//! 输入框原生右键编辑菜单：剪切 / 复制 / 粘贴 / 全选。
//!
//! 弹出位置由 `Window::popup_menu` 自动取光标坐标（免前端传参 + DPR 转换）；
//! 菜单项一律用系统预定义项（`PredefinedMenuItem`）——文案、快捷键提示由 OS
//! 本地化，点击行为作用于当前聚焦的可编辑元素，与 Finder / 资源管理器右键一致。
//! 后端自定义命令不经 capability 校验，故无需在 capabilities 声明 core:menu:*。

use tauri::{
    menu::{Menu, PredefinedMenuItem},
    Window,
};

#[tauri::command]
pub async fn show_edit_menu(window: Window) -> Result<(), String> {
    let cut = PredefinedMenuItem::cut(&window, None).map_err(|e| format!("弹出编辑菜单失败：{e}"))?;
    let copy = PredefinedMenuItem::copy(&window, None).map_err(|e| format!("弹出编辑菜单失败：{e}"))?;
    let paste = PredefinedMenuItem::paste(&window, None).map_err(|e| format!("弹出编辑菜单失败：{e}"))?;
    let select_all = PredefinedMenuItem::select_all(&window, None).map_err(|e| format!("弹出编辑菜单失败：{e}"))?;
    let menu = Menu::with_items(&window, &[&cut, &copy, &paste, &select_all])
        .map_err(|e| format!("弹出编辑菜单失败：{e}"))?;
    window.popup_menu(&menu).map_err(|e| format!("弹出编辑菜单失败：{e}"))?;
    Ok(())
}
