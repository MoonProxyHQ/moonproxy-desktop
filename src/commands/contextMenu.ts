import { invoke } from "@tauri-apps/api/core";

/**
 * 弹出原生编辑菜单（剪切 / 复制 / 粘贴 / 全选），作用于当前聚焦的可编辑元素。
 * 仅应在 input / textarea / select 的 contextmenu 事件中调用。
 */
export async function showEditMenu(): Promise<string | null> {
  try {
    await invoke("show_edit_menu");
    return null;
  } catch (e: any) {
    return typeof e === "string" ? e : e?.message ?? "菜单弹出失败";
  }
}
