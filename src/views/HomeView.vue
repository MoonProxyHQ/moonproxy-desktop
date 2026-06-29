<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { Settings } from "@lucide/vue";

import { config, isConfigured, frpcStatus } from "../state";
import { startFrpc, stopFrpc } from "../commands/frpc";
import ControlBar from "../components/home/ControlBar.vue";
import ProxyList from "../components/home/ProxyList.vue";
import GuideCard from "../components/home/GuideCard.vue";
import SystemStatus from "../components/home/SystemStatus.vue";

const { t: $t } = useI18n();
const emit = defineEmits<{ settings: [] }>();

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
    <button
      class="home-settings-btn"
      @click="emit('settings')"
      :title="$t('home_settings_title')"
      :aria-label="$t('home_settings_title')"
    >
      <Settings :size="18" />
    </button>
    <ControlBar :disabled="!isConfigured()" @click="onToggle" />
    <div class="home-body">
      <GuideCard v-if="!isConfigured()" @settings="emit('settings')" />
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
  position: relative;
}

/* 系统设置齿轮：浮在 ControlBar 卡片右上角内侧（与卡片 padding 对齐） */
.home-settings-btn {
  position: absolute;
  top: 25px;
  right: 14px;
  z-index: 10;
  width: 30px;
  height: 30px;
  border-radius: 6px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: hsl(var(--muted-foreground));
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}
.home-settings-btn:hover {
  background: hsl(var(--accent));
  color: hsl(var(--foreground));
}

.home-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  /* 滚动不传染外层；触底不弹跳 */
  overscroll-behavior: contain;
}

/* 错误提示卡片：与列表卡片节奏一致 */
.error-msg {
  color: hsl(var(--destructive));
  font-size: 12px;
  padding: 10px 14px;
  background-color: hsl(var(--destructive) / 0.06);
  border: 1px solid hsl(var(--destructive) / 0.2);
  border-radius: var(--radius);
  font-weight: 500;
}
</style>