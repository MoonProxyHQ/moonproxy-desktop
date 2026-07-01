<script setup lang="ts">
import type { ComponentPublicInstance } from "vue";
import { nextTick, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Trash2 } from "@lucide/vue";

import { config } from "../../state";
import { saveConfig } from "../../commands/config";
import type { ProxyConfig, ProxyType } from "../../types";
import { useToast } from "../../composables/useToast";
import Toast from "../Toast.vue";

const { t: $t } = useI18n();
const { toast, showToast } = useToast();

/** 代理规则数量上限；达到后隐藏「添加代理」按钮。 */
const MAX_PROXIES = 50;

/** 校验失败时定位到的字段名（与 ProxyForm 对应）。 */
type FieldName = "name" | "local_ip" | "local_port" | "remote_port" | "custom_domain";

/** 校验错误：包含代理下标（1-based）+ 字段名 + 提示文案。
 *  主公要求：校验失败时把焦点移到对应输入框、外边框变红——需要知道「哪个
 *  代理的哪个字段错了」才能精准定位与高亮。 */
interface ValidationError {
  n: number;
  field: FieldName;
  message: string;
}

/** 当前出错的字段（null 表示无错误）。用户修改任意字段时清空。 */
const fieldError = ref<{ n: number; field: FieldName } | null>(null);

/** 每个代理各字段的 DOM 句柄，用于校验失败时 `focus()`。 */
const inputRefs = reactive<Record<string, HTMLElement | null>[]>([]);

/**
 * 表单中间类型：所有可能的字段都铺平在同一结构里。
 *
 * 设计动机：Vue 的 `v-model` 在 discriminated union 上不友好（每次绑定都要
 * 先 type-narrow）。所以表单内部用一个胖结构，保存时再按 `type` 拆成对应
 * 的 `ProxyConfig` variant——不合法字段在转换阶段被丢弃，序列化到后端的
 * 永远是 union 形态。
 *
 * 字段含义：
 * - `remote_port`：tcp/udp 用；http/https 时该字段不展示，提交时丢弃
 * - `custom_domain`：http/https 用；单值域名；tcp/udp 时该字段不展示
 */
interface ProxyForm {
  name: string;
  type: ProxyType;
  local_ip: string;
  local_port: number;
  remote_port: number;
  custom_domain: string;
}

/** 把 reactive `config.proxies`（ProxyConfig union）映射为表单副本。 */
function toForm(p: ProxyConfig): ProxyForm {
  const base = {
    name: p.name,
    type: p.type,
    local_ip: p.local_ip,
    local_port: p.local_port,
  };
  switch (p.type) {
    case "tcp":
    case "udp":
      return { ...base, remote_port: p.remote_port, custom_domain: "" };
    case "http":
    case "https":
      return {
        ...base,
        remote_port: 0,
        custom_domain: p.custom_domain,
      };
  }
}

const form = reactive({
  proxies: (config.proxies.length
    ? config.proxies.map(toForm)
    : [
        {
          name: "",
          type: "tcp" as ProxyType,
          local_ip: "127.0.0.1",
          local_port: 80,
          remote_port: 0,
          custom_domain: "",
        },
      ]) as ProxyForm[],
});

const saving = ref(false);

function onlyNumber(e: KeyboardEvent) {
  if (e.ctrlKey || e.metaKey) return;
  const allowed = ['Backspace', 'Delete', 'ArrowLeft', 'ArrowRight', 'Tab', 'Enter'];
  if (!/^\d$/.test(e.key) && !allowed.includes(e.key)) {
    e.preventDefault();
  }
}

function addProxy() {
  form.proxies.push({
    name: "",
    type: "tcp",
    local_ip: "127.0.0.1",
    local_port: 80,
    remote_port: 0,
    custom_domain: "",
  });
}

function removeProxy(index: number) {
  form.proxies.splice(index, 1);
}

/** 是否为端口型代理（TCP/UDP），需要 `remote_port` 字段。 */
function isPortProxy(t: ProxyType): boolean {
  return t === "tcp" || t === "udp";
}

/**
 * 域名格式校验：每段（label）字母/数字/连字符、不以连字符开头/结尾；
 * 至少两个 label。不做 DNS 真实存在性检查——避免阻塞网络。
 */
const DOMAIN_LABEL_RE = /^[a-zA-Z0-9](?:[a-zA-Z0-9-]*[a-zA-Z0-9])?$/;
function isValidDomain(s: string): boolean {
  if (!s || s.length > 253) return false;
  const labels = s.split(".");
  if (labels.length < 2) return false;
  return labels.every((l) => l.length >= 1 && l.length <= 63 && DOMAIN_LABEL_RE.test(l));
}

/** 把校验失败的第一个字段定位出来（含代理下标 + 字段名 + 文案）。 */
function validate(): ValidationError | null {
  if (form.proxies.length === 0) {
    return { n: 0, field: "name", message: $t("proxy_err_min") };
  }
  for (let i = 0; i < form.proxies.length; i++) {
    const n = i + 1;
    const p = form.proxies[i];
    if (!p.name.trim()) return { n, field: "name", message: $t("proxy_err_incomplete", { n }) };
    if (!p.local_ip.trim()) return { n, field: "local_ip", message: $t("proxy_err_incomplete", { n }) };
    if (p.local_port <= 0) return { n, field: "local_port", message: $t("proxy_err_port", { n }) };
    if (isPortProxy(p.type)) {
      if (p.remote_port <= 0) return { n, field: "remote_port", message: $t("proxy_err_port", { n }) };
    } else {
      const d = p.custom_domain.trim();
      if (!d) return { n, field: "custom_domain", message: $t("proxy_err_custom_domain", { n }) };
      if (!isValidDomain(d)) return { n, field: "custom_domain", message: $t("proxy_err_domain_format", { n }) };
    }
  }
  return null;
}

/** 把函数 ref 注册到对应 (代理下标, 字段名)，方便校验失败时 focus。
 *  Vue 3 函数 ref 签名要求参数兼容 `Element | ComponentPublicInstance | null`。 */
function setInputRef(i: number, field: FieldName) {
  return (el: Element | ComponentPublicInstance | null) => {
    if (!inputRefs[i]) inputRefs[i] = {};
    inputRefs[i][field] = (el as HTMLElement | null) ?? null;
  };
}

/** 某字段是否当前处于校验失败态——用于决定是否加红边框。 */
function isInvalid(i: number, field: FieldName): boolean {
  return fieldError.value?.n === i + 1 && fieldError.value.field === field;
}

/** 用户改动任意字段时清掉当前代理的错误高亮（避免红边框滞留误导）。 */
function clearErrorForIndex(i: number) {
  if (fieldError.value?.n === i + 1) fieldError.value = null;
}

/** 表单副本 → ProxyConfig union，按 `type` 拆出对应字段。 */
function fromForm(p: ProxyForm): ProxyConfig {
  const name = p.name.trim();
  const local_ip = p.local_ip.trim();
  const local_port = Number(p.local_port);
  switch (p.type) {
    case "tcp":
    case "udp":
      return {
        type: p.type,
        name,
        local_ip,
        local_port,
        remote_port: Number(p.remote_port),
      };
    case "http":
    case "https":
      return {
        type: p.type,
        name,
        local_ip,
        local_port,
        custom_domain: p.custom_domain.trim(),
      };
  }
}

async function onSave() {
  const err = validate();
  if (err) {
    fieldError.value = { n: err.n, field: err.field };
    showToast(err.message, "error");
    // 等 v-model 把 class 应用到 DOM 后再聚焦（红边框 + 焦点同步出现）
    nextTick(() => {
      const el = inputRefs[err.n - 1]?.[err.field];
      el?.focus();
    });
    return;
  }
  fieldError.value = null;
  saving.value = true;
  config.proxies = form.proxies.map(fromForm);
  const e = await saveConfig();
  saving.value = false;
  if (e) showToast($t("msg_save_failed", { err: e }), "error", 4000);
  else showToast($t("msg_save_success"), "success", 1200);
}
</script>

<template>
  <div class="tab-pane">
    <section class="card section-card">
      <div class="section-title">{{ $t("proxy_section_title") }}</div>
      <div v-for="(p, i) in form.proxies" :key="i" class="proxy-card">
        <div class="proxy-head">
          <span class="proxy-idx">#{{ i + 1 }}</span>
          <button class="btn btn-ghost btn-sm btn-icon" @click="removeProxy(i)" :title="$t('proxy_remove')" :aria-label="$t('proxy_remove')">
            <Trash2 :size="15" style="color:hsl(var(--destructive))" />
          </button>
        </div>
        <div class="proxy-grid">
          <label class="form-item">
            <span class="label">{{ $t("proxy_label_type") }}</span>
            <select
              class="input proxy-type"
              v-model="p.type"
              @change="fieldError = null"
            >
              <option value="tcp">TCP</option>
              <option value="udp">UDP</option>
              <option value="http">HTTP</option>
              <option value="https">HTTPS</option>
            </select>
          </label>
          <label class="form-item span-3">
            <span class="label">{{ $t("proxy_label_name") }}</span>
            <input
              class="input"
              :class="{ 'is-invalid': isInvalid(i, 'name') }"
              :ref="setInputRef(i, 'name')"
              v-model="p.name"
              :placeholder="$t('proxy_ph_name')"
              @input="clearErrorForIndex(i)"
            />
          </label>
          <label class="form-item" :class="{ 'span-2': isPortProxy(p.type), 'span-3': !isPortProxy(p.type) }">
            <span class="label">{{ $t("proxy_label_local_ip") }}</span>
            <input
              class="input"
              :class="{ 'is-invalid': isInvalid(i, 'local_ip') }"
              :ref="setInputRef(i, 'local_ip')"
              v-model="p.local_ip"
              :placeholder="$t('proxy_ph_local_ip')"
              @input="clearErrorForIndex(i)"
            />
          </label>
          <label class="form-item">
            <span class="label">{{ $t("proxy_label_local_port") }}</span>
            <input
              class="input"
              :class="{ 'is-invalid': isInvalid(i, 'local_port') }"
              :ref="setInputRef(i, 'local_port')"
              v-model.number="p.local_port"
              type="number"
              min="1"
              max="65535"
              :placeholder="$t('proxy_ph_local_port')"
              @keydown="onlyNumber"
              @input="clearErrorForIndex(i)"
            />
          </label>
          <label v-if="isPortProxy(p.type)" class="form-item">
            <span class="label">{{ $t("proxy_label_remote_port") }}</span>
            <input
              class="input"
              :class="{ 'is-invalid': isInvalid(i, 'remote_port') }"
              :ref="setInputRef(i, 'remote_port')"
              v-model.number="p.remote_port"
              type="number"
              min="1"
              max="65535"
              :placeholder="$t('proxy_ph_remote_port')"
              @keydown="onlyNumber"
              @input="clearErrorForIndex(i)"
            />
          </label>
          <label v-else class="form-item span-4">
            <span class="label">{{ $t("proxy_label_custom_domain") }}</span>
            <input
              class="input"
              :class="{ 'is-invalid': isInvalid(i, 'custom_domain') }"
              :ref="setInputRef(i, 'custom_domain')"
              v-model="p.custom_domain"
              :placeholder="$t('proxy_ph_custom_domain')"
              @input="clearErrorForIndex(i)"
            />
          </label>
        </div>
        <div v-if="!isPortProxy(p.type)" class="proxy-hint">
          {{ $t("proxy_hint_http_domain") }}
        </div>
      </div>
      <div class="proxy-add-row">
        <button
          v-if="form.proxies.length < MAX_PROXIES"
          class="btn btn-outline btn-sm"
          @click="addProxy"
        >{{ $t("proxy_add") }}</button>
      </div>
    </section>

    <footer class="tab-footer">
      <button class="btn btn-primary" @click="onSave" :disabled="saving">
        {{ saving ? $t("common_saving") : $t("common_save") }}
      </button>
    </footer>

    <Toast :toast="toast" />
  </div>
</template>

<style scoped>
.tab-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.section-card {
  padding: 14px;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 12px;
}
.form-item {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.form-item.span-2 {
  grid-column: span 2;
}
.form-item.span-3 {
  grid-column: span 3;
}
.form-item.span-4 {
  grid-column: span 4;
}
.label {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  font-weight: 500;
}
.proxy-card {
  background-color: hsl(var(--muted) / 0.5);
  border: 1px solid hsl(var(--border));
  border-radius: calc(var(--radius) - 2px);
  padding: 10px;
  margin-bottom: 8px;
}
.proxy-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 10px;
}
.proxy-idx {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  font-weight: 600;
  min-width: 24px;
}
.proxy-type {
  appearance: none;
  padding-right: 28px;
  background-image: linear-gradient(45deg, transparent 50%, hsl(var(--muted-foreground)) 50%),
    linear-gradient(135deg, hsl(var(--muted-foreground)) 50%, transparent 50%);
  background-position: calc(100% - 14px) 50%, calc(100% - 9px) 50%;
  background-size: 5px 5px;
  background-repeat: no-repeat;
}
.proxy-grid {
  display: grid;
  grid-template-columns: 1.4fr 1fr 1fr 1fr;
  gap: 8px;
}
/* 校验失败时输入框红色外边框 + 同色微光，提醒用户 */
.input.is-invalid {
  border-color: hsl(var(--destructive));
  box-shadow: 0 0 0 2px hsl(var(--destructive) / 0.18);
}
.input.is-invalid:focus {
  outline: none;
  border-color: hsl(var(--destructive));
  box-shadow: 0 0 0 3px hsl(var(--destructive) / 0.3);
}
.proxy-hint {
  margin-top: 8px;
  font-size: 11px;
  line-height: 1.5;
  color: hsl(var(--muted-foreground));
  padding: 0 2px;
}
.proxy-add-row {
  display: flex;
  justify-content: flex-start;
  margin-top: 4px;
}
.tab-footer {
  display: flex;
  justify-content: flex-end;
  padding: 12px 0 20px;
}
</style>
