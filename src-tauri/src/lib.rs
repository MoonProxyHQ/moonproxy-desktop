//! Tauri 应用入口：Builder 配置 + setup hook + 命令注册 + 退出兜底。
//!
//! 业务实现按职责拆分到子模块：
//! - `types`：StartArgs / ProxyConfig 共享类型
//! - `config`：frpc.toml 生成 + 配置持久化命令
//! - `process`：frpc 子进程生命周期（启动 / 停止 / 状态 / 日志 / 孤儿清理）
//! - `frpc_state`：连接状态机 + 日志环形缓冲
//! - `frpc_update`：frpc 引擎自更新
//! - `proxy_health`：代理本地端口连通性探测
//! - `prefs`：应用偏好（开机启动 / 静默启动 / 开机自动连接 / 定时连接）
//! - `tray`：系统托盘

use tauri::{Manager, RunEvent};
use tauri_plugin_autostart::MacosLauncher;

use config::{load_config, save_config};
use context_menu::show_edit_menu;
use frpc_state::FrpcState;
use frpc_update::{
    apply_pending_frpc_update, apply_pending_update, check_frpc_update, download_frpc_update,
    get_frpc_version,
};
use prefs::{get_auto_launch, get_prefs, maybe_auto_connect, maybe_silent_start, save_prefs, set_auto_launch};
use latency::probe_server_latency;
use process::{frpc_running, frpc_status, get_logs, start_frpc, stop_frpc};
use proxy_health::check_proxies_health;
use tray::init_tray;

mod config;
mod context_menu;
mod frpc_state;
mod frpc_update;
mod latency;
mod prefs;
mod process;
mod proxy_health;
mod scheduler;
mod tray;
mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--auto-launched"]),
        ))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .manage(FrpcState::default())
        .setup(|app| {
            if let Err(e) = apply_pending_update(app.handle()) {
                eprintln!("[frpc-update] 应用待安装更新失败: {e}");
            }
            // setup 顶层共享 prefs + auto_launched：避免 maybe_silent_start /
            // maybe_auto_connect / scheduler::maybe_catch_up_start 各自再次
            // load 同一份 prefs.json + 扫一遍 argv（详见 src-tauri/AGENTS.md §5.2.1）
            let app_handle = app.handle();
            let loaded_prefs = prefs::load(app_handle);
            let auto_launched = std::env::args().any(|a| a == "--auto-launched");
            maybe_silent_start(app_handle, &loaded_prefs, auto_launched);
            maybe_auto_connect(app_handle, &loaded_prefs, auto_launched);
            scheduler::maybe_catch_up_start(app_handle, &loaded_prefs);
            scheduler::spawn_scheduler(app_handle.clone());
            init_tray(app_handle)?;

            // macOS WKWebView 启动时默认不抢焦点：首次点击被 WebKit 消费为
            // 「窗口聚焦」事件，不会派发 DOM click，用户表现为「单击无效需双击」。
            // 静默启动已 hide 的窗口不能 set_focus（会把窗口从托盘里拽出来），
            // 故仅在窗口当前可见时调用。unwrap_or(true)：取不到状态时默认尝试
            // set_focus（首次双击启动场景下无害）。
            if let Some(window) = app.get_webview_window("main") {
                if window.is_visible().unwrap_or(true) {
                    let _ = window.set_focus();
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_frpc,
            stop_frpc,
            frpc_running,
            frpc_status,
            save_config,
            load_config,
            check_proxies_health,
            probe_server_latency,
            get_logs,
            get_frpc_version,
            check_frpc_update,
            download_frpc_update,
            apply_pending_frpc_update,
            get_prefs,
            save_prefs,
            set_auto_launch,
            get_auto_launch,
            show_edit_menu
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            // 应用即将退出（托盘"退出" / Cmd+Q / 关闭确认"退出" / 最后窗口关闭
            // 等路径都汇聚到此）。无论哪条路径，都先同步 kill 掉 frpc 子进程，
            // 避免主进程退出后留下孤儿 frpc。
            //
            // 注意：SIGKILL / SIGTERM 等外部信号**不会**触发本钩子，需要靠
            // `process::reap_orphan_frpc` 在下次启动时兜底清理（详见
            // src-tauri/AGENTS.md §5.3 SIGKILL 失效场景）。
            if let RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app_handle.try_state::<FrpcState>() {
                    // mutex 中毒（dev 环境 panic 常见）时不要静默吞：取 inner
                    // 取回仍可访问，同时记录以便排查。
                    let child = state
                        .child
                        .lock()
                        .unwrap_or_else(|e| {
                            eprintln!("[exit] FrpcState.child mutex 中毒：{e}");
                            e.into_inner()
                        })
                        .take();
                    if let Some(child) = child {
                        let _ = child.kill();
                    }
                }
            }
        });
}
