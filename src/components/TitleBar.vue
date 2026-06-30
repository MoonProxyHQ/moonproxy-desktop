<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "vue-i18n";
import { ArrowLeft, Minus, Server, Settings, X } from "@lucide/vue";
import BrandIcon from "./BrandIcon.vue";

type View = "home" | "settings" | "services";

const { t } = useI18n();

const props = defineProps<{ view: View }>();
const emit = defineEmits<{ back: []; settings: []; services: [] }>();

const isMac = navigator.userAgent.includes("Mac");
const isWin = navigator.userAgent.includes("Win");

async function minimize() {
  await getCurrentWindow().minimize();
}
async function closeWin() {
  await getCurrentWindow().close();
}
</script>

<template>
  <header class="titlebar" :class="{ 'titlebar-mac': isMac, 'titlebar-win': isWin }" data-tauri-drag-region>
    <!-- 左槽：macOS 给交通灯让位 78px；非首页视图（Windows）显示返回按钮；home 视图（Windows）渲染服务/设置（远离右侧窗口控件） -->
    <div class="slot-left">
      <button
        v-if="props.view !== 'home' && isWin"
        class="btn btn-ghost btn-icon drag-exclude"
        @click="emit('back')"
        :title="t('common_back')"
        :aria-label="t('common_back')"
      >
        <ArrowLeft :size="18" />
      </button>
      <div v-else-if="isWin" class="titlebar-actions">
        <button class="btn btn-ghost btn-icon drag-exclude" @click="emit('services')">
          <Server :size="16" />
        </button>
        <button class="btn btn-ghost btn-icon drag-exclude" @click="emit('settings')">
          <Settings :size="16" />
        </button>
      </div>
    </div>

    <!-- 中间：拖动区 + 标题（绝对居中于窗口） -->
    <div class="slot-center">
      <span v-if="props.view === 'home'" class="brand">
        <BrandIcon :size="14" />
        <span class="brand-name">{{ t("app_name") }}</span>
      </span>
      <span v-else-if="props.view === 'services'" class="view-title">{{ t("services_view_title") }}</span>
      <span v-else class="view-title">{{ t("settings_view_title") }}</span>
    </div>

    <!-- 右槽：macOS home 视图入口 / macOS 非 home 视图返回按钮 + Windows 窗口控制 -->
    <div class="slot-right">
      <div v-if="props.view === 'home' && isMac" class="titlebar-actions">
        <button class="btn btn-ghost btn-icon drag-exclude" @click="emit('services')">
          <Server :size="16" />
        </button>
        <button class="btn btn-ghost btn-icon drag-exclude" @click="emit('settings')">
          <Settings :size="16" />
        </button>
      </div>
      <button
        v-else-if="props.view !== 'home' && isMac"
        class="btn btn-ghost btn-icon drag-exclude"
        @click="emit('back')"
        :title="t('common_back')"
        :aria-label="t('common_back')"
      >
        <ArrowLeft :size="18" />
      </button>
      <div v-if="isWin" class="win-controls">
        <button
          class="win-btn win-btn-min drag-exclude"
          @click="minimize"
          :title="t('common_minimize')"
          :aria-label="t('common_minimize')"
        >
          <Minus :size="14" :stroke-width="2.5" />
        </button>
        <button
          class="win-btn win-btn-close drag-exclude"
          @click="closeWin"
          :title="t('common_close')"
          :aria-label="t('common_close')"
        >
          <X :size="14" :stroke-width="2.5" />
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  height: 38px;
  display: grid;
  grid-template-columns: var(--left-slot, 1px) 1fr var(--right-slot, 1px);
  align-items: center;
  user-select: none;
  flex-shrink: 0;
  border-bottom: 1px solid hsl(var(--border));
  cursor: default;
}
.titlebar button {
  cursor: default;
}
.titlebar .btn-icon {
  padding: 4px;
}
/* Home 视图右槽入口：紧凑水平排列 + 居中对齐，与标题栏同高 */
.titlebar-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  height: 100%;
}
.titlebar-mac {
  --left-slot: 78px; /* 给 macOS 交通灯让位 */
  padding-left: 0;
  padding-right: 8px;
}
.titlebar-win {
  --right-slot: auto;
  padding-left: 8px;
  padding-right: 0;
}

.slot-left,
.slot-right {
  display: flex;
  align-items: center;
  height: 100%;
  grid-row: 1;
  position: relative;
  z-index: 2;
}
.slot-left {
  grid-column: 1;
  justify-content: flex-start;
}
.slot-right {
  grid-column: 3;
  justify-content: flex-end;
  gap: 4px;
}

.slot-center {
  grid-column: 1 / -1;
  grid-row: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  z-index: 1;
  pointer-events: none;
}
/* drag region 由父级 .titlebar 提供，交互子元素单独退出 */
.drag-exclude {
  -webkit-app-region: no-drag;
  pointer-events: auto;
}

/* Home 标题：图标 + 应用名 */
.brand {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: hsl(var(--foreground));
}
.brand-name {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.2px;
}

/* Settings 标题 */
.view-title {
  font-size: 13px;
  font-weight: 600;
}

/* Windows 窗口控制按钮 */
.win-controls {
  display: flex;
  height: 100%;
}
.win-btn {
  width: 46px;
  height: 100%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: hsl(var(--foreground));
  transition: background-color 0.15s;
  -webkit-app-region: no-drag;
}
.win-btn:hover {
  background-color: hsl(var(--accent));
}
/* 关闭按钮独立配色 */
.win-btn-close:hover {
  background-color: hsl(var(--destructive));
  color: hsl(var(--destructive-foreground));
}
</style>
