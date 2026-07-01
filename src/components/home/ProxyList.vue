<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
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

/** 退避档位：3s → 6s → 12s → 24s，4 档后封顶（3 次倍增） */
const HEALTH_INTERVAL_MIN = 3000;
const HEALTH_INTERVAL_MAX = 24000;

const copiedIndex = ref<number | null>(null);
let copiedTimer: ReturnType<typeof setTimeout> | null = null;
let healthTimer: ReturnType<typeof setTimeout> | null = null;
/** 健康检测指数退避调度状态（实例级闭包变量，不进响应式） */
let healthPrevSig: string | null = null;
let healthStreak = 0;
/** 退避调度代际：每次 reset/unmount/新 tickHealth 入口自增；
 * await 中的旧 tickHealth 完成时若代际已变即 return，避免并发循环。
 * 范式对齐后端 frpc_state::poll_gen（src-tauri/AGENTS.md §5.5）。 */
let healthGen = 0;

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

/** 把当前 proxyHealth 拼成签名串：所有代理 ok 值一致才算稳定。 */
function healthSignature(): string {
  return proxyHealth.value.map((h) => (h ? (h.ok ? "T" : "F") : "?")).join(",");
}

/** 按 streak 算下次间隔：3s·2^streak，封顶 24s */
function nextHealthInterval(streak: number): number {
  return Math.min(HEALTH_INTERVAL_MIN * 2 ** streak, HEALTH_INTERVAL_MAX);
}

/** 一轮探测 + 调度下一轮。整体稳定（签名不变）则递增 streak 升档；翻转则归零回 3s。
 *  代际机制保证：await 期间若有 reset / unmount / 新一轮 tickHealth 进入，
 *  本实例 await 返回后立即 return，不重复注册 timer（避免并发循环泄漏）。 */
async function tickHealth() {
  const myGen = ++healthGen;
  await checkProxiesHealth();
  if (myGen !== healthGen) return;
  const sig = healthSignature();
  if (healthPrevSig !== null && sig === healthPrevSig) {
    healthStreak += 1;
  } else {
    healthStreak = 0;
  }
  healthPrevSig = sig;
  healthTimer = setTimeout(tickHealth, nextHealthInterval(healthStreak));
}

/** 重置退避：bump 代际（让飞行中的 tickHealth 退出）+ 清挂起定时器 + 清状态。
 *  调用方负责随后主动 tickHealth() 启动新一轮；空配置场景则不再启动。 */
function resetHealthBackoff() {
  healthGen += 1;
  if (healthTimer) {
    clearTimeout(healthTimer);
    healthTimer = null;
  }
  healthPrevSig = null;
  healthStreak = 0;
}

onMounted(() => {
  // 立即跑一次，随后按指数退避节奏自调度
  tickHealth();
});

/** 代理列表增/删/改：先 reset（含 bump 代际，让飞行中的 tickHealth 退出）；
 *  非空配置则立即重检（自然落到 3s 起步档）；空配置则仅 reset、不启新循环，
 *  避免 proxies 删空后旧 tickHealth 以封顶 24s 节奏空转。 */
watch(
  () => props.proxies,
  () => {
    resetHealthBackoff();
    if (!proxyEndpoints.value.length) return;
    void tickHealth();
  },
  { deep: true },
);

onUnmounted(() => {
  healthGen += 1;
  if (copiedTimer) clearTimeout(copiedTimer);
  if (healthTimer) clearTimeout(healthTimer);
});
</script>

<template>
  <section v-if="proxyEndpoints.length" class="endpoints-section">
    <div class="endpoints-title">{{ $t("home_endpoints_title") }}</div>
    <div class="endpoints-scroll">
      <div
        v-for="(ep, i) in proxyEndpoints"
        :key="i"
        class="endpoint-row"
        :class="{ connected: frpcStatus === 'connected' }"
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
    </div>
  </section>
</template>

<style scoped>
.endpoints-section {
  /* 占据流量卡片与启动按钮之间的剩余空间：标题固定在顶，
     端点行在 .endpoints-scroll 内部滚动。min-height:0 是 flex 子项可收缩的关键。 */
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.endpoints-title {
  font-size: 12px;
  font-weight: 600;
  color: hsl(var(--muted-foreground));
  margin-bottom: 8px;
  padding: 0 4px;
  flex-shrink: 0;
}
.endpoints-scroll {
  /* 端点行的独立滚动容器：少量端点 safe center 垂直居中；
     端点溢出时 safe 回退顶部对齐并内部滚动，标题与外层卡片/按钮始终固定。 */
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  justify-content: safe center;
  overflow-y: auto;
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
