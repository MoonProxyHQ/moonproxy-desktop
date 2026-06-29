<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Check, Copy } from "@lucide/vue";

import type { ProxyConfig } from "../../types";
import { frpcStatus } from "../../state/runtime";
import {
  proxyHealth,
  checkProxiesHealth,
} from "../../composables/useProxyHealth";

const props = defineProps<{
  proxies: ProxyConfig[];
  serverAddr: string;
}>();

const { t: $t } = useI18n();

/** 密度档位阈值。
 *
 * - ≤ COMPACT_THRESHOLD：双行宽松布局（name / url 各占一行）
 * - > COMPACT_THRESHOLD：紧凑单行布局，提升单屏容量
 * - > COLLAPSE_THRESHOLD：在紧凑模式基础上折叠剩余条目，留出兜底入口
 */
const COMPACT_THRESHOLD = 3;
const COLLAPSE_THRESHOLD = 12;
const COLLAPSE_VISIBLE = 8;

/** 生成每条代理的公网访问地址。
 *
 * - tcp/udp：`<server_addr>:<remote_port>`——frp 直接转发端口
 * - http/https：`<type>://<custom_domain>`——经 frps vhost 按域名路由分发
 */
const proxyEndpoints = computed(() => {
  if (!props.serverAddr) return [];
  return props.proxies.map((p) => {
    switch (p.type) {
      case "tcp":
      case "udp":
        return { name: p.name, url: `${props.serverAddr}:${p.remote_port}` };
      case "http":
      case "https":
        return { name: p.name, url: `${p.type}://${p.custom_domain}` };
    }
  });
});

const copiedIndex = ref<number | null>(null);
let copiedTimer: ReturnType<typeof setTimeout> | null = null;
let healthTimer: ReturnType<typeof setInterval> | null = null;

const isCompact = computed(() => proxyEndpoints.value.length > COMPACT_THRESHOLD);
const collapsible = computed(() => proxyEndpoints.value.length > COLLAPSE_THRESHOLD);
const expanded = ref(false);
const visibleEndpoints = computed(() => {
  if (!collapsible.value || expanded.value) return proxyEndpoints.value;
  return proxyEndpoints.value.slice(0, COLLAPSE_VISIBLE);
});
const hiddenCount = computed(() =>
  collapsible.value && !expanded.value
    ? proxyEndpoints.value.length - COLLAPSE_VISIBLE
    : 0,
);

function copyText(text: string, index: number) {
  navigator.clipboard?.writeText(text);
  copiedIndex.value = index;
  if (copiedTimer) clearTimeout(copiedTimer);
  copiedTimer = setTimeout(() => {
    copiedIndex.value = null;
    copiedTimer = null;
  }, 1200);
}

/** 代理行下标对应的健康状态（可能为 undefined 表示尚未检测） */
function healthFor(i: number) {
  return proxyHealth.value[i];
}
/** 状态点的 CSS 类名：未检测 / 可达 / 异常 三态 */
function healthClass(i: number) {
  const h = healthFor(i);
  if (!h) return "dot-pending";
  return h.ok ? "dot-ok" : "dot-fail";
}
/** 拼接给状态点 title / aria-label 的文案；前缀描述结论，后缀附 message */
function healthTitle(i: number) {
  const h = healthFor(i);
  if (!h) return $t("home_endpoint_health_pending");
  return $t(
    h.ok ? "home_endpoint_health_ok" : "home_endpoint_health_fail",
    { msg: h.message },
  );
}
/** 仅当确实检测过且不可达时返回 true；用于代理行高亮等装饰逻辑 */
function isFailed(i: number): boolean {
  const h = healthFor(i);
  return !!h && !h.ok;
}

onMounted(() => {
  // 立即跑一次，再每 3 秒轮询
  checkProxiesHealth();
  healthTimer = setInterval(checkProxiesHealth, 3000);
});

onUnmounted(() => {
  if (copiedTimer) clearTimeout(copiedTimer);
  if (healthTimer) clearInterval(healthTimer);
});
</script>

<template>
  <section v-if="proxyEndpoints.length" class="endpoints-section">
    <div class="endpoints-title">{{ $t("home_endpoints_title") }}</div>
    <div
      v-for="(ep, i) in visibleEndpoints"
      :key="i"
      class="endpoint-row"
      :class="{
        connected: frpcStatus === 'connected',
        compact: isCompact,
      }"
    >
      <div class="endpoint-meta">
        <span class="endpoint-name" :class="{ 'name-fail': isFailed(i) }">
          <span
            class="health-dot"
            :class="healthClass(i)"
            :title="healthTitle(i)"
            :aria-label="healthTitle(i)"
          ></span>
          <span class="endpoint-name-text">{{ ep.name }}</span><span
            v-if="isFailed(i)"
            class="endpoint-reason"
            :title="healthFor(i)?.message"
          >（{{ healthFor(i)?.message }}）</span>
        </span>
        <span class="endpoint-url mono" :title="ep.url">{{ ep.url }}</span>
      </div>
      <button
        class="copy-btn"
        :class="{ copied: copiedIndex === i }"
        :title="copiedIndex === i ? $t('home_endpoint_copied') : $t('home_endpoint_copy')"
        :aria-label="copiedIndex === i ? $t('home_endpoint_copied') : $t('home_endpoint_copy_aria', { url: ep.url })"
        @click="copyText(ep.url, i)"
      >
        <Check v-if="copiedIndex === i" :size="14" :stroke-width="2.5" />
        <Copy v-else :size="14" />
      </button>
    </div>
    <button
      v-if="collapsible"
      class="endpoint-collapse-btn"
      @click="expanded = !expanded"
    >
      {{
        expanded
          ? $t("home_endpoint_collapse")
          : $t("home_endpoint_expand_more", { count: hiddenCount })
      }}
    </button>
  </section>
</template>

<style scoped>
.endpoints-section {
  display: flex;
  flex-direction: column;
  padding: 12px;
  background: hsl(var(--card));
  border: 1px solid hsl(var(--border));
  border-radius: var(--radius);
}
.endpoints-title {
  font-size: 12px;
  font-weight: 600;
  color: hsl(var(--muted-foreground));
  margin-bottom: 8px;
  padding: 0 2px;
}
.endpoint-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: calc(var(--radius) - 2px);
  background: hsl(var(--secondary) / 0.4);
  margin-bottom: 6px;
}
.endpoint-row.connected {
  background: hsl(var(--success) / 0.12);
}
.endpoint-row:last-child {
  margin-bottom: 0;
}
.endpoint-meta {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.endpoint-name {
  font-size: 13px;
  color: hsl(var(--muted-foreground));
  font-weight: 500;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.endpoint-name.name-fail {
  color: hsl(var(--warning));
  font-weight: 600;
}
.endpoint-reason {
  font-size: 11px;
  font-weight: 500;
  opacity: 0.85;
}

/* 紧凑模式：单行布局，name 左、url 自适应中段（截断保留 title 悬浮） */
.endpoint-row.compact {
  padding: 6px 10px;
  margin-bottom: 4px;
}
.endpoint-row.compact .endpoint-meta {
  flex-direction: row;
  align-items: center;
  gap: 8px;
}
.endpoint-row.compact .endpoint-name {
  flex-shrink: 0;
  max-width: 45%;
  overflow: hidden;
  white-space: nowrap;
  flex-wrap: nowrap;
}
.endpoint-row.compact .endpoint-name-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.endpoint-row.compact .endpoint-url {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

/* 折叠兜底按钮：条目超出阈值时收起剩余，点击就地展开 */
.endpoint-collapse-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  padding: 8px 12px;
  margin-top: 2px;
  border: 1px dashed hsl(var(--border));
  border-radius: calc(var(--radius) - 2px);
  background: transparent;
  color: hsl(var(--muted-foreground));
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s, border-color 0.15s;
}
.endpoint-collapse-btn:hover {
  background: hsl(var(--accent));
  color: hsl(var(--foreground));
  border-color: hsl(var(--accent-foreground) / 0.3);
}
.health-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
  background: hsl(var(--muted-foreground) / 0.4);
  transition: background-color 0.3s, box-shadow 0.3s;
}
.health-dot.dot-ok {
  background: hsl(var(--success));
  box-shadow: 0 0 5px hsl(var(--success) / 0.5);
}
.health-dot.dot-fail {
  background: hsl(var(--destructive));
  box-shadow: 0 0 5px hsl(var(--destructive) / 0.5);
  animation: dot-blink 1.2s ease-in-out infinite;
}
.health-dot.dot-pending {
  background: hsl(var(--muted-foreground) / 0.35);
}
@keyframes dot-blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
.endpoint-url {
  font-size: 12px;
  color: hsl(var(--foreground));
  font-weight: 500;
  user-select: text;
  -webkit-user-select: text;
  cursor: text;
  word-break: break-all;
}
.mono {
  font-family: "SF Mono", Menlo, Consolas, monospace;
}
.copy-btn {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid hsl(var(--border));
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: background-color 0.15s, border-color 0.15s, color 0.15s;
}
.copy-btn:hover {
  background: hsl(var(--accent));
  color: hsl(var(--foreground));
  border-color: hsl(var(--accent-foreground) / 0.3);
}
.copy-btn.copied {
  background: hsl(var(--success) / 0.12);
  border-color: hsl(var(--success) / 0.3);
  color: hsl(var(--success));
}
</style>
