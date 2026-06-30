<script setup lang="ts">
import { ref } from "vue";

import { config, isConfigured, frpcStatus } from "../state";
import { startFrpc, stopFrpc } from "../commands/frpc";
import CircleButton from "../components/home/CircleButton.vue";
import ProxyList from "../components/home/ProxyList.vue";
import GuideCard from "../components/home/GuideCard.vue";
import SystemStatus from "../components/home/SystemStatus.vue";

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
      <CircleButton :disabled="!isConfigured()" @click="onToggle" />
      <GuideCard v-if="!isConfigured()" @services="emit('services')" />
      <ProxyList
        :proxies="config.proxies"
        :server-addr="config.server_addr"
      />
      <div v-if="error" class="error-msg">{{ error }}</div>
    </div>
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
  overflow-y: auto;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  /* 滚动不传染外层；触底不弹跳（none：阻链式 + 阻弹跳）*/
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
