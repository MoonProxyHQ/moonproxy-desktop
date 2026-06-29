<script setup lang="ts">
import type { ComponentPublicInstance } from "vue";
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { Eye, EyeOff } from "@lucide/vue";
import { config } from "../../state";
import { saveConfig } from "../../commands/config";
import { useToast } from "../../composables/useToast";
import Toast from "../Toast.vue";

const { t: $t } = useI18n();
const { toast, showToast } = useToast();

const form = reactive({
  custom_name: config.custom_name ?? "",
  server_addr: config.server_addr,
  server_port: config.server_port,
  token: config.token ?? "",
  user: config.user ?? "",
});

const saving = ref(false);
const showPassword = ref(false);
function togglePassword() {
  showPassword.value = !showPassword.value;
}

/** 服务端延迟测试结果，对应后端 `latency::LatencyResult` */
interface LatencyResult {
  ok: boolean;
  latency_ms: number;
  error_kind: string | null;
}

type LatencyStatus = "idle" | "testing" | "ok" | "fail";

const latencyStatus = ref<LatencyStatus>("idle");
const latencyMs = ref<number | null>(null);
const latencyErrorKind = ref<string | null>(null);

const latencyText = computed(() => {
  switch (latencyStatus.value) {
    case "testing":
      return $t("provider_testing");
    case "ok":
      return $t("provider_test_ok", { ms: latencyMs.value ?? 0 });
    case "fail":
      switch (latencyErrorKind.value) {
        case "empty":
          return $t("provider_test_err_empty");
        case "resolve":
          return $t("provider_test_fail_resolve");
        case "timeout":
          return $t("provider_test_fail_timeout");
        case "refused":
          return $t("provider_test_fail_refused");
        default:
          return $t("provider_test_fail_unreachable");
      }
    default:
      return "";
  }
});

const latencyClass = computed(() => {
  if (latencyStatus.value === "ok") return "latency-ok";
  if (latencyStatus.value === "fail") return "latency-fail";
  return "latency-pending";
});

async function onTestLatency() {
  if (latencyStatus.value === "testing") return;
  const addr = form.server_addr.trim();
  const port = Number(form.server_port);
  if (!addr || !port || port <= 0) {
    latencyStatus.value = "fail";
    latencyErrorKind.value = "empty";
    return;
  }
  latencyStatus.value = "testing";
  try {
    const result = await invoke<LatencyResult>("probe_server_latency", {
      serverAddr: addr,
      serverPort: port,
    });
    if (result.ok) {
      latencyStatus.value = "ok";
      latencyMs.value = result.latency_ms;
      latencyErrorKind.value = null;
    } else {
      latencyStatus.value = "fail";
      latencyErrorKind.value = result.error_kind ?? "unreachable";
    }
  } catch (e) {
    latencyStatus.value = "fail";
    latencyErrorKind.value = "unreachable";
    console.warn("[latency] probe_server_latency failed:", e);
  }
}

// 地址 / 端口变化时清空旧结果，避免显示与当前输入不一致的延迟数字。
watch(
  () => [form.server_addr, form.server_port],
  () => {
    if (latencyStatus.value !== "idle") {
      latencyStatus.value = "idle";
      latencyMs.value = null;
      latencyErrorKind.value = null;
    }
  },
);

function onlyNumber(e: KeyboardEvent) {
  if (e.ctrlKey || e.metaKey) return;
  const allowed = ["Backspace", "Delete", "ArrowLeft", "ArrowRight", "Tab", "Enter"];
  if (!/^\d$/.test(e.key) && !allowed.includes(e.key)) {
    e.preventDefault();
  }
}

/** 可校验字段：与 ProxyTab 的 FieldName 同形——校验失败时聚焦 + 红边框 */
type FieldName = "custom_name" | "server_addr" | "server_port" | "user";

interface ValidationError {
  field: FieldName;
  message: string;
}

/** 当前出错的字段（null 表示无错误）。用户修改任意字段时清空。 */
const fieldError = ref<{ field: FieldName } | null>(null);

/** 各字段 DOM 句柄，用于校验失败时 `focus()`。 */
const inputRefs = reactive<Partial<Record<FieldName, HTMLElement | null>>>({});

function setInputRef(field: FieldName) {
  return (el: Element | ComponentPublicInstance | null) => {
    inputRefs[field] = (el as HTMLElement | null) ?? null;
  };
}

function isInvalid(field: FieldName): boolean {
  return fieldError.value?.field === field;
}

function clearError(field: FieldName) {
  if (fieldError.value?.field === field) fieldError.value = null;
}

function validate(): ValidationError | null {
  if (!form.custom_name.trim()) return { field: "custom_name", message: $t("provider_err_custom_name") };
  if (!form.server_addr.trim()) return { field: "server_addr", message: $t("provider_err_server_addr") };
  if (!form.server_port || form.server_port <= 0) return { field: "server_port", message: $t("provider_err_server_port") };
  if (!form.user.trim()) return { field: "user", message: $t("provider_err_user") };
  return null;
}

async function onSave() {
  const err = validate();
  if (err) {
    fieldError.value = { field: err.field };
    showToast(err.message, "error");
    nextTick(() => {
      inputRefs[err.field]?.focus();
    });
    return;
  }
  fieldError.value = null;
  saving.value = true;
  config.custom_name = form.custom_name.trim();
  config.server_addr = form.server_addr.trim();
  config.server_port = Number(form.server_port);
  config.token = form.token.trim();
  config.user = form.user.trim();
  const e = await saveConfig();
  saving.value = false;
  if (e) showToast($t("msg_save_failed", { err: e }), "error", 4000);
  else showToast($t("msg_save_success"), "success", 1200);
}

/**
 * 从 frpc 配置片段解析服务端连接字段。
 *
 * 仅识别 frpc 官方驼峰平铺格式（serverAddr / serverPort / auth.token / user）；
 * 老式下划线（server_addr）与嵌套 `[auth]` 块不识别——主公明确选了严格模式，
 * 减少歧义与误识别。
 *
 * 返回值：成功返回对象，失败返回面向用户的错误文案。
 */
interface ParsedServerConfig {
  server_addr: string;
  server_port: number;
  token: string;
  user: string;
  /** 可选：用户在配置片段里用 `serverName` 指定的服务商名称。 */
  server_name?: string;
}

// 行匹配用 `m` 标志按行扫描；等号两侧空格、行首行尾空格都容忍。
// 字符串值用双引号包裹（TOML 规范），端口必须是裸数字。
const RE_NAME = /^\s*serverName\s*=\s*"([^"]*)"\s*$/m;
const RE_ADDR = /^\s*serverAddr\s*=\s*"([^"]*)"\s*$/m;
const RE_PORT = /^\s*serverPort\s*=\s*(\d+)\s*$/m;
const RE_TOKEN = /^\s*auth\.token\s*=\s*"([^"]*)"\s*$/m;
const RE_USER = /^\s*user\s*=\s*"([^"]*)"\s*$/m;

function parseServerConfig(text: string): ParsedServerConfig | string {
  const name = text.match(RE_NAME);
  const addr = text.match(RE_ADDR);
  const port = text.match(RE_PORT);
  const token = text.match(RE_TOKEN);
  const user = text.match(RE_USER);

  if (!addr) return $t("provider_import_err_no_addr");
  if (!port) return $t("provider_import_err_no_port");
  if (!token) return $t("provider_import_err_no_token");
  if (!user) return $t("provider_import_err_no_user");

  const server_addr = addr[1].trim();
  const server_port = Number(port[1]);

  if (!server_addr) return $t("provider_import_err_no_addr");
  if (!Number.isInteger(server_port) || server_port < 1 || server_port > 65535) {
    return $t("provider_import_err_port_invalid");
  }

  const server_name = name ? name[1].trim() : "";
  return {
    server_addr,
    server_port,
    token: token[1],
    user: user[1],
    ...(server_name ? { server_name } : {}),
  };
}

const showImportModal = ref(false);
const importText = ref("");
const importError = ref<string | null>(null);

function openImport() {
  importText.value = "";
  importError.value = null;
  showImportModal.value = true;
}

function closeImport() {
  showImportModal.value = false;
  importError.value = null;
}

function confirmImport() {
  const result = parseServerConfig(importText.value);
  if (typeof result === "string") {
    importError.value = result;
    return;
  }
  // serverName 优先：用户在配置片段里显式指定了服务商名称就直接采用；
  // 否则保留已填名称，仍为空才回退到默认文案。
  if (result.server_name) {
    form.custom_name = result.server_name;
  } else if (!form.custom_name.trim()) {
    form.custom_name = $t("provider_import_default_name");
  }
  form.server_addr = result.server_addr;
  form.server_port = result.server_port;
  form.token = result.token;
  form.user = result.user;
  importError.value = null;
  showImportModal.value = false;
  showToast($t("provider_import_ok"), "success", 2500);
}

function onImportKeydown(e: KeyboardEvent) {
  if (!showImportModal.value) return;
  if (e.key === "Escape") {
    e.preventDefault();
    closeImport();
  } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
    e.preventDefault();
    confirmImport();
  }
}

onMounted(() => window.addEventListener("keydown", onImportKeydown));
onUnmounted(() => window.removeEventListener("keydown", onImportKeydown));
</script>

<template>
  <div class="tab-pane">
    <section class="card section-card">
      <div class="section-title">{{ $t("provider_section_title") }}</div>
      <div class="form-grid">
        <label class="form-item span-2">
          <span class="label">{{ $t("provider_label_custom_name") }}</span>
          <input
            class="input"
            :class="{ 'is-invalid': isInvalid('custom_name') }"
            :ref="setInputRef('custom_name')"
            v-model="form.custom_name"
            :placeholder="$t('provider_ph_custom_name')"
            maxlength="32"
            @input="clearError('custom_name')"
          />
        </label>

        <label class="form-item">
          <span class="label">{{ $t("provider_label_server_addr") }}</span>
          <input
            class="input"
            :class="{ 'is-invalid': isInvalid('server_addr') }"
            :ref="setInputRef('server_addr')"
            v-model="form.server_addr"
            :placeholder="$t('provider_ph_server_addr')"
            @input="clearError('server_addr')"
          />
        </label>
        <label class="form-item">
          <span class="label">{{ $t("provider_label_server_port") }}</span>
          <input
            class="input"
            :class="{ 'is-invalid': isInvalid('server_port') }"
            :ref="setInputRef('server_port')"
            v-model.number="form.server_port"
            type="number"
            min="1"
            max="65535"
            @keydown="onlyNumber"
            @input="clearError('server_port')"
          />
        </label>
        <label class="form-item">
          <span class="label">{{ $t("provider_label_user") }}</span>
          <input
            class="input"
            :class="{ 'is-invalid': isInvalid('user') }"
            :ref="setInputRef('user')"
            v-model="form.user"
            :placeholder="$t('provider_ph_user_required')"
            @input="clearError('user')"
          />
        </label>
        <div class="form-item span-2">
          <span class="label">{{ $t("provider_label_token") }}</span>
          <div class="password-field">
            <input
              class="input password-input"
              v-model="form.token"
              :type="showPassword ? 'text' : 'password'"
              :placeholder="$t('provider_ph_token')"
              autocomplete="off"
            />
            <button
              type="button"
              class="password-toggle"
              :aria-label="showPassword ? $t('provider_hide_password') : $t('provider_show_password')"
              :title="showPassword ? $t('provider_hide_password') : $t('provider_show_password')"
              @click="togglePassword"
            >
              <EyeOff v-if="showPassword" :size="16" aria-hidden="true" />
              <Eye v-else :size="16" aria-hidden="true" />
            </button>
          </div>
        </div>
      </div>
    </section>

    <footer class="tab-footer">
      <div class="footer-left">
        <button
          type="button"
          class="btn btn-outline"
          @click="openImport"
        >
          {{ $t('provider_btn_import') }}
        </button>
        <button
          type="button"
          class="btn btn-outline"
          :disabled="latencyStatus === 'testing'"
          @click="onTestLatency"
        >
          {{ latencyStatus === 'testing' ? $t('provider_testing') : $t('provider_btn_test') }}
        </button>
        <span
          v-if="latencyText"
          class="latency-result"
          :class="latencyClass"
        >{{ latencyText }}</span>
      </div>
      <button class="btn btn-primary" @click="onSave" :disabled="saving">
        {{ saving ? $t("common_saving") : $t("common_save") }}
      </button>
    </footer>

    <Teleport to="body">
      <div
        v-if="showImportModal"
        class="import-mask"
        @click.self="closeImport"
      >
        <div
          class="import-card"
          role="dialog"
          aria-modal="true"
          aria-labelledby="import-title"
        >
          <button
            class="import-x"
            type="button"
            :aria-label="$t('common_cancel')"
            :title="$t('common_cancel')"
            @click="closeImport"
          >×</button>
          <div id="import-title" class="import-title">
            {{ $t('provider_import_title') }}
          </div>
          <p class="import-desc">{{ $t('provider_import_desc') }}</p>
          <textarea
            v-model="importText"
            class="import-textarea"
            :placeholder="$t('provider_import_ph')"
            spellcheck="false"
            autocomplete="off"
            autofocus
          ></textarea>
          <p v-if="importError" class="import-error">{{ importError }}</p>
          <div class="import-actions">
            <button type="button" class="btn btn-outline" @click="closeImport">
              {{ $t('common_cancel') }}
            </button>
            <button type="button" class="btn btn-primary" @click="confirmImport">
              {{ $t('common_save') }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>

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
.form-grid {
  display: grid;
  grid-template-columns: 3fr 1fr;
  gap: 10px;
}
.form-item {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.form-item.span-2 {
  grid-column: span 2;
}
.label {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  font-weight: 500;
}
/* 校验失败时输入框红色外边框 + 同色微光——与 ProxyTab 的 .input.is-invalid 视觉一致 */
.input.is-invalid {
  border-color: hsl(var(--destructive));
  box-shadow: 0 0 0 2px hsl(var(--destructive) / 0.18);
}
.input.is-invalid:focus {
  outline: none;
  border-color: hsl(var(--destructive));
  box-shadow: 0 0 0 3px hsl(var(--destructive) / 0.3);
}
.password-field {
  position: relative;
  display: flex;
  align-items: center;
}
.password-input {
  padding-right: 32px;
}
.password-toggle {
  position: absolute;
  right: 4px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  border: none;
  background: transparent;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  border-radius: calc(var(--radius) - 2px);
  transition: color 0.15s;
}
.password-toggle:hover {
  color: hsl(var(--foreground));
}
.password-toggle:focus-visible {
  outline: none;
  box-shadow: 0 0 0 3px hsl(var(--ring) / 0.12);
}
.tab-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  padding: 12px 0 0;
}
.footer-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.latency-result {
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.latency-ok {
  color: hsl(var(--success));
}
.latency-fail {
  color: hsl(var(--destructive));
}
.latency-pending {
  color: hsl(var(--muted-foreground));
}

/* 从 frpc 配置导入弹窗：与 CloseConfirm 共用遮罩 + 卡片视觉语言 */
.import-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}
.import-card {
  position: relative;
  width: 420px;
  max-width: calc(100vw - 32px);
  background: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: var(--radius);
  padding: 20px 20px 16px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.2);
}
.import-x {
  position: absolute;
  top: 6px;
  right: 6px;
  background: transparent;
  border: none;
  color: hsl(var(--muted-foreground));
  font-size: 18px;
  line-height: 1;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: default;
}
.import-x:hover {
  background: hsl(var(--accent));
}
.import-title {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 8px;
  padding-right: 24px;
}
.import-desc {
  font-size: 12px;
  line-height: 1.6;
  color: hsl(var(--muted-foreground));
  margin: 0 0 10px;
}
.import-textarea {
  width: 100%;
  min-height: 180px;
  max-height: 360px;
  padding: 8px 10px;
  border: 1px solid hsl(var(--border));
  border-radius: calc(var(--radius) - 2px);
  background: hsl(var(--background));
  color: hsl(var(--foreground));
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 12px;
  line-height: 1.5;
  resize: vertical;
  outline: none;
  box-sizing: border-box;
}
.import-textarea:focus {
  border-color: hsl(var(--ring));
  box-shadow: 0 0 0 3px hsl(var(--ring) / 0.12);
}
.import-error {
  font-size: 12px;
  color: hsl(var(--destructive));
  margin: 8px 0 0;
}
.import-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 14px;
}
</style>
