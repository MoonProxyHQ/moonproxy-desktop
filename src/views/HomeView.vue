<script setup lang="ts">
import { ref } from "vue";

import { config, isConfigured, frpcStatus } from "../state";
import { startFrpc, stopFrpc } from "../commands/frpc";
import StartButton from "../components/home/StartButton.vue";
import ProxyList from "../components/home/ProxyList.vue";
import GuideCard from "../components/home/GuideCard.vue";
import SystemStatus from "../components/home/SystemStatus.vue";
import TrafficChart from "../components/home/TrafficChart.vue";

const emit = defineEmits<{ services: [] }>();

const error = ref("");

async function onToggle() {
  error.value = "";
  const s = frpcStatus.value;
  if (s === "connecting" || s === "connected") {
    const err = await stopFrpc();
    if (err) error.value = err;
  } else {
    // stopped / error：先确保没有残留子进程，再启动
    if (s === "error") {
      await stopFrpc().catch(() => undefined);
    }
    const err = await startFrpc();
    if (err) error.value = err;
  }
}
</script>

<template>
  <div class="home-view">
    <div class="home-body">
      <TrafficChart />
      <GuideCard v-if="!isConfigured()" @services="emit('services')" />
      <ProxyList
        :proxies="config.proxies"
        :server-addr="config.server_addr"
      />
      <div v-if="error" class="error-msg">{{ error }}</div>
    </div>
    <StartButton :disabled="!isConfigured()" @click="onToggle" />
    <SystemStatus />
  </div>
</template>

<style scoped>
.home-view {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.home-body {
  flex: 1;
  /* 本层不滚动：顶部流量卡片固定高度、中部端点列表在自身容器内滚动，
     底部启动按钮 + 状态栏始终可见。 */
  overflow: hidden;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overscroll-behavior: none;
}

.error-msg {
  color: hsl(var(--destructive));
  font-size: 12px;
  padding: 8px 12px;
  background-color: hsl(var(--destructive) / 0.08);
  border-radius: calc(var(--radius) - 2px);
}
</style>
