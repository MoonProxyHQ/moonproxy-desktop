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
import { showEditMenu } from "./commands/contextMenu";
import Toast from "./components/Toast.vue";
import { useToast } from "./composables/useToast";

type View = "home" | "settings" | "services";
const currentView = ref<View>("home");

const { showCloseConfirm } = useAppEvents();
const { toast, showToast } = useToast();

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

// 防 async 弹菜单期间重复右键导致并发 popup_menu
let editMenuPending = false;

/** 桌面化右键：可编辑元素弹原生编辑菜单，其余区域屏蔽浏览器默认菜单 */
async function onContextMenu(e: MouseEvent) {
  const t = e.target;
  if (
    t instanceof HTMLInputElement ||
    t instanceof HTMLTextAreaElement ||
    t instanceof HTMLSelectElement ||
    (t instanceof HTMLElement && t.isContentEditable)
  ) {
    // 阻止 WebView 默认菜单，避免与原生菜单双弹
    e.preventDefault();
    if (editMenuPending) return; // 防重入
    editMenuPending = true;
    try {
      // 同步聚焦右键目标，确保 PredefinedMenuItem 作用于它而非旧焦点
      t.focus();
      const err = await showEditMenu();
      if (err) showToast(err, "error", 2500);
    } finally {
      editMenuPending = false;
    }
    return;
  }
  e.preventDefault();
}

onMounted(() => {
  window.addEventListener("keydown", onKeydown);
  window.addEventListener("contextmenu", onContextMenu);
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  window.removeEventListener("contextmenu", onContextMenu);
});
</script>

<template>
  <div class="app-root">
    <TitleBar :view="currentView" @back="goHome" @settings="goSettings" @services="goServices" />
    <UpdateBanners @install="onInstallApp" />
    <HomeView v-if="currentView === 'home'" @services="goServices" />
    <ServicesView v-else-if="currentView === 'services'" @back="goHome" />
    <SettingsView v-else @back="goHome" />
    <CloseConfirm v-model="showCloseConfirm" />
    <Toast :toast="toast" />
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
