import { ref, shallowRef } from "vue";
import { check, type Update, type DownloadEvent } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

import { i18n } from "../i18n";

export interface AppUpdateInfo {
  /** 当前版本（来自 build 时注入，与 tauri.conf.json 同步） */
  currentVersion: string;
  /** 最新版本号 */
  version: string;
  /** 发行说明（来自 latest.json 的 body 字段） */
  notes?: string;
  /** 发行日期 */
  date?: string;
}

/** 应用本体版本号（三处同步：`tauri.conf.json` / `package.json` / `Cargo.toml`）；本文件即 `APP_VERSION` 的定义位置 */
export const APP_VERSION = "1.1.0";

/** 检测到的新版本；null 表示无更新或尚未检查 */
export const appUpdateAvailable = ref<AppUpdateInfo | null>(null);
/**
 * 已下载、待用户点「重启并安装」的 Update 句柄；null 表示无待安装。
 *
 * 必须用 shallowRef：`Update` 类内部含 `#xxx` 私有字段（V8 强约束，
 * 不允许跨 Proxy 边界访问）。普通 `ref` 会深度代理 `.value` 内部对象，
 * 导致 `install()` 访问私有字段时抛
 * "Cannot read private member from an object whose class did not declare it"。
 * shallowRef 只保留引用身份，不代理对象内部结构。
 */
export const appUpdatePending = shallowRef<Update | null>(null);
/** 是否正在检查更新 */
export const appUpdateChecking = ref(false);
/** 是否正在下载（含进度 0-100） */
export const appUpdateDownloading = ref(false);
export const appUpdateProgress = ref(0);

/** 从最新版本切换回「无更新」的 helper */
function clearAvailable() {
  appUpdateAvailable.value = null;
  // 旧 Update 句柄若存在，关掉释放资源
  if (appUpdatePending.value) {
    appUpdatePending.value.close().catch(() => undefined);
    appUpdatePending.value = null;
  }
}

/** 后台静默检查 GitHub 是否有新版本 */
export async function checkAppUpdate(): Promise<void> {
  appUpdateChecking.value = true;
  try {
    const upd = await check();
    if (upd) {
      appUpdateAvailable.value = {
        currentVersion: upd.currentVersion,
        version: upd.version,
        notes: upd.body,
        date: upd.date,
      };
    } else {
      clearAvailable();
    }
  } catch (e) {
    console.warn("[app-update] 检查失败", e);
  } finally {
    appUpdateChecking.value = false;
  }
}

/**
 * 下载最新版本到本地缓存（不重启）。
 * 成功后 appUpdatePending 置位；返回 null；失败返回错误字符串。
 *
 * 注意 Tauri 2.10 的 update.download 是「下载完先回调 Finished 事件、再 await resolve」
 * 的顺序，UI 必须依赖 Finished 事件而不是 await 完成来切到「重启并安装」状态，
 * 否则在某些环境下会出现「进度 100% 后 UI 卡住」。
 */
export async function downloadAppUpdate(): Promise<string | null> {
  if (!appUpdateAvailable.value) return i18n.global.t("err_no_app_update");
  appUpdateDownloading.value = true;
  appUpdateProgress.value = 0;

  // 闭包状态：用于按 chunkLength / contentLength 计算真实百分比
  let receivedBytes = 0;
  let contentLength: number | undefined;

  try {
    let upd = appUpdatePending.value;
    if (!upd) {
      upd = await check();
      if (!upd) return i18n.global.t("err_no_app_update_found");
    }

    // Finished 事件触发时立即把 pending 标记置上，避免 invoke 延迟返回时
    // UI 卡在「下载中 100%」；await 返回后再补一次保险。
    let finished = false;

    await upd.download((ev: DownloadEvent) => {
      if (ev.event === "Started") {
        contentLength = ev.data.contentLength;
        receivedBytes = 0;
        appUpdateProgress.value = 0;
      } else if (ev.event === "Progress") {
        receivedBytes += ev.data.chunkLength;
        if (contentLength && contentLength > 0) {
          appUpdateProgress.value = Math.min(
            100,
            Math.floor((receivedBytes / contentLength) * 100),
          );
        } else {
          // 无 contentLength 时退化到粗粒度 +1；99 封顶避免提前显示完成
          appUpdateProgress.value = Math.min(99, appUpdateProgress.value + 1);
        }
      } else if (ev.event === "Finished") {
        finished = true;
        appUpdateProgress.value = 100;
        appUpdatePending.value = upd;
      }
    });

    if (!finished) {
      // 极少数情况：没收到 Finished 事件但 invoke 已返回
      appUpdatePending.value = upd;
    }
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? i18n.global.t("err_download");
  } finally {
    appUpdateDownloading.value = false;
  }
}

/**
 * 安装已下载的更新并重启应用。
 * 失败返回错误字符串。
 */
export async function installAppUpdate(): Promise<string | null> {
  const upd = appUpdatePending.value;
  if (!upd) return i18n.global.t("err_no_update_downloaded");
  try {
    await upd.install();
    await relaunch();
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? i18n.global.t("err_install");
  }
}
