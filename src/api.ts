import { invoke } from "@tauri-apps/api/core";
import type { CaptureResult, ClipItemSummary, WindowInfo } from "./types";
import type { AppSettings } from "./types/settings";

export const getHistory = () => invoke<ClipItemSummary[]>("get_history");
export const pinItem = (id: string) => invoke<void>("pin_item", { id });
export const unpinItem = (id: string) => invoke<void>("unpin_item", { id });
export const deleteItem = (id: string) => invoke<void>("delete_item", { id });
export const clearUnpinned = () => invoke<void>("clear_unpinned");
export const getItemContent = (id: string) =>
  invoke<string>("get_item_content", { id });
export const updateItemText = (id: string, content: string) =>
  invoke<void>("update_item_text", { id, content });
export const copyItemToClipboard = (id: string) =>
  invoke<void>("copy_item_to_clipboard", { id });
export const pasteItemToTarget = (id: string) =>
  invoke<void>("paste_item_to_target", { id });
export const copyTextToClipboard = (text: string) =>
  invoke<void>("copy_text_to_clipboard", { text });
export const pasteTextToTarget = (text: string) =>
  invoke<void>("paste_text_to_target", { text });
export const listCaptureWindows = () =>
  invoke<WindowInfo[]>("list_capture_windows");
export const snipFullscreen = () => invoke<CaptureResult>("snip_fullscreen");
export const snipWindow = (windowId: number) =>
  invoke<CaptureResult>("snip_window", { windowId });
export const snipRegion = (x: number, y: number, width: number, height: number) =>
  invoke<CaptureResult>("snip_region", { x, y, width, height });
export const copyPngToClipboard = (pngBase64: string) =>
  invoke<void>("copy_png_to_clipboard", { pngBase64 });
export const savePng = (pngBase64: string, filename?: string) =>
  invoke<string>("save_png", { pngBase64, filename });
export const saveEditedSnip = (pngBase64: string, path?: string | null) =>
  invoke<CaptureResult>("save_edited_snip", { pngBase64, path: path ?? null });
export const openSnipEditor = (capture?: CaptureResult | null) =>
  invoke<void>("open_snip_editor", { capture: capture ?? null });
export const openSnipEditorFromHistory = (id: string) =>
  invoke<void>("open_snip_editor_from_history", { id });
export const getLastSnipCapture = () =>
  invoke<CaptureResult | null>("get_last_snip_capture");
export const getSettings = () => invoke<AppSettings>("get_settings");
export const setSettings = (settings: AppSettings) =>
  invoke<void>("set_settings", { settings });
export const openKeyboardShortcuts = () => invoke<void>("open_keyboard_shortcuts");
export const showSettings = () => invoke<void>("show_settings");