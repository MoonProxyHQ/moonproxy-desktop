<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  frpcVersion,
  updateInfo,
  downloadedPending,
  downloading,
  checkFrpcUpdate,
  downloadFrpcUpdate,
} from "../../composables/useFrpcUpdate";
import {
  APP_VERSION,
  appUpdateAvailable,
  appUpdatePending,
  appUpdateChecking,
  appUpdateDownloading,
  appUpdateProgress,
  checkAppUpdate,
  downloadAppUpdate,
  installAppUpdate,
} from "../../composables/useAppUpdate";
import { useToast } from "../../composables/useToast";
import Toast from "../Toast.vue";

const { t: $t } = useI18n();
const { toast, showToast } = useToast();

const frpcChecking = ref(false);
const frpcError = ref("");

async function onCheckFrpcUpdate() {
  frpcChecking.value = true;
  frpcError.value = "";
  try {
    await checkFrpcUpdate();
    if (!updateInfo.value) {
      showToast($t("msg_engine_latest"), "success", 1500);
    }
  } finally {
    frpcChecking.value = false;
  }
}

async function onDownloadFrpcUpdate() {
  if (!updateInfo.value) return;
  frpcError.value = "";
  const err = await downloadFrpcUpdate(updateInfo.value.latest_version);
  if (err) {
    showToast($t("msg_download_failed", { err }), "error", 4000);
  } else {
    showToast($t("msg_engine_download_ok"), "success", 2000);
  }
}

async function onCheckAppUpdate() {
  try {
    await checkAppUpdate();
    if (!appUpdateAvailable.value) {
      showToast($t("msg_app_latest"), "success", 1500);
    }
  } catch {
    /* 静默；错误已在 composable 内 console.warn */
  }
}

async function onDownloadAppUpdate() {
  const err = await downloadAppUpdate();
  if (err) {
    showToast($t("msg_download_failed", { err }), "error", 4000);
  } else {
    showToast($t("msg_app_download_ok"), "success", 2500);
  }
}

async function onInstallAppUpdate() {
  const err = await installAppUpdate();
  if (err) {
    showToast($t("msg_install_failed", { err }), "error", 4000);
  }
  // 成功路径下 relaunch 会重启进程，下方代码不会执行
}
</script>

<template>
  <div class="tab-pane">
    <!-- 软件本体 -->
    <section class="card section-card">
      <div class="section-title">{{ $t("updates_section_app") }}</div>
      <div class="engine-info">
        <div class="engine-row">
          <span class="label">{{ $t("updates_label_current_version") }}</span>
          <span class="engine-value engine-value-current">v{{ APP_VERSION }}</span>
        </div>
        <div v-if="appUpdateAvailable" class="engine-row">
          <span class="label">{{ $t("updates_label_latest_version") }}</span>
          <span class="engine-value engine-value-latest">v{{ appUpdateAvailable.version }}</span>
        </div>
        <div v-if="appUpdatePending" class="engine-row">
          <span class="label">{{ $t("updates_label_downloaded") }}</span>
          <span class="engine-value">v{{ appUpdateAvailable?.version }}{{ $t("updates_value_pending_app") }}</span>
        </div>
        <div v-if="appUpdateDownloading && appUpdateProgress > 0" class="engine-row">
          <span class="label">{{ $t("updates_label_progress") }}</span>
          <span class="engine-value">{{ appUpdateProgress }}%</span>
        </div>
      </div>
      <div class="engine-actions">
        <button
          v-if="!appUpdateAvailable && !appUpdatePending"
          class="btn btn-outline btn-sm"
          :disabled="appUpdateChecking"
          @click="onCheckAppUpdate"
        >
          {{ appUpdateChecking ? $t("updates_btn_checking") : $t("updates_btn_check") }}
        </button>
        <button
          v-if="appUpdateAvailable && !appUpdatePending"
          class="btn btn-primary btn-sm"
          :disabled="appUpdateDownloading"
          @click="onDownloadAppUpdate"
        >
          {{ appUpdateDownloading
              ? $t("updates_btn_downloading_app", { progress: appUpdateProgress })
              : $t("updates_btn_download_app") }}
        </button>
        <button
          v-if="appUpdatePending"
          class="btn btn-primary btn-sm"
          @click="onInstallAppUpdate"
        >
          {{ $t("updates_btn_install") }}
        </button>
      </div>
    </section>

    <!-- 核心引擎 -->
    <section class="card section-card">
      <div class="section-title">{{ $t("updates_section_engine") }}</div>
      <div class="engine-info">
        <div class="engine-row">
          <span class="label">{{ $t("updates_label_current_version") }}</span>
          <span class="engine-value">v{{ frpcVersion || "—" }}</span>
        </div>
        <div v-if="updateInfo" class="engine-row">
          <span class="label">{{ $t("updates_label_latest_version") }}</span>
          <span class="engine-value">v{{ updateInfo.latest_version }}</span>
        </div>
        <div v-if="downloadedPending" class="engine-row">
          <span class="label">{{ $t("updates_label_downloaded") }}</span>
          <span class="engine-value">v{{ downloadedPending }}{{ $t("updates_value_pending_engine") }}</span>
        </div>
        <div v-if="frpcError" class="engine-status">{{ frpcError }}</div>
      </div>
      <div class="engine-actions">
        <button
          v-if="!updateInfo && !downloadedPending"
          class="btn btn-outline btn-sm"
          :disabled="frpcChecking"
          @click="onCheckFrpcUpdate"
        >
          {{ frpcChecking ? $t("updates_btn_checking") : $t("updates_btn_check") }}
        </button>
        <button
          v-if="updateInfo"
          class="btn btn-primary btn-sm"
          :disabled="downloading"
          @click="onDownloadFrpcUpdate"
        >
          {{ downloading
              ? $t("updates_btn_downloading_engine")
              : $t("updates_btn_download_engine") }}
        </button>
      </div>
    </section>

    <Toast :toast="toast" />
  </div>
</template>

<style scoped>
.tab-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  gap: 12px;
}
.section-card {
  padding: 14px;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 12px;
}
.engine-info {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.engine-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
}
.label {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
  font-weight: 500;
}
.engine-value {
  font-family: "SF Mono", Menlo, Consolas, monospace;
  font-weight: 500;
}
/* 当前版本：弱化（muted-foreground）；最新版本：success 强调（绿色与「升级成功」语义一致） */
.engine-value-current {
  color: hsl(var(--muted-foreground));
}
.engine-value-latest {
  color: hsl(var(--success));
  font-weight: 600;
}
.engine-status {
  font-size: 12px;
  color: hsl(var(--muted-foreground));
}
.engine-actions {
  margin-top: 12px;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
