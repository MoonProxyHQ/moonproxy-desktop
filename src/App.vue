<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

import TitleBar from "./components/TitleBar.vue";
import HomeView from "./views/HomeView.vue";
import SettingsView from "./views/SettingsView.vue";
import ServicesView from "./views/ServicesView.vue";
import CloseConfirm from "./components/CloseConfirm.vue";
import UpdateBanners from "./components/banners/UpdateBanners.vue";
import { useAppEvents } from "./composables/useAppEvents";
import { installAppUpdate } from "./composables/useAppUpdate";

type View = "home" | "settings" | "services";
const currentView = ref<View>("home");

const { showCloseConfirm } = useAppEvents();

function goSettings() {
  currentView.value = "settings";
}

function goServices() {
  currentView.value = "services";
}

function goHome() {
  currentView.value = "home";
}

/** 用户点击横幅上的「重启并安装」按钮 */
async function onInstallApp() {
  const err = await installAppUpdate();
  if (err) {
    console.warn("[app-update] 安装失败", err);
  }
}

/** 全局键盘快捷键（桌面应用惯例） */
function onKeydown(e: KeyboardEvent) {
  // 关闭确认弹窗打开期间，让弹窗独占 Esc / Enter，避免与下方快捷键冲突
  if (showCloseConfirm.value) return;
  const mod = e.metaKey || e.ctrlKey;
  // Cmd/Ctrl + W 关闭窗口
  if (mod && e.key.toLowerCase() === "w") {
    e.preventDefault();
    getCurrentWindow().close();
    return;
  }
  // Cmd/Ctrl + M 最小化
  if (mod && e.key.toLowerCase() === "m") {
    e.preventDefault();
    getCurrentWindow().minimize();
    return;
  }
  // Esc：非首页视图（设置 / 服务）返回 home
  if (e.key === "Escape" && currentView.value !== "home") {
    e.preventDefault();
    goHome();
  }
}

/** 屏蔽浏览器原生右键菜单（桌面应用惯例） */
function onContextMenu(e: MouseEvent) {
  e.preventDefault();
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown, { passive: true });
  window.addEventListener("contextmenu", onContextMenu, { passive: true });
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("contextmenu", onContextMenu);
});
</script>

<template>
  <div class="app-root">
    <TitleBar :view="currentView" @back="goHome" />
    <UpdateBanners @install="onInstallApp" />
    <HomeView v-if="currentView === 'home'" @settings="goSettings" @services="goServices" />
    <ServicesView v-else-if="currentView === 'services'" @back="goHome" />
    <SettingsView v-else @back="goHome" />
    <CloseConfirm v-model="showCloseConfirm" />
  </div>
</template>

<style scoped>
.app-root {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}
</style>
