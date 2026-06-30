import { invoke } from "@tauri-apps/api/core";
import type { CaptureResult, ClipItemSummary, WindowInfo } from "./types";

export const getHistory = () => invoke<ClipItemSummary[]>("get_history");
export const pinItem = (id: string) => invoke<void>("pin_item", { id });
export const unpinItem = (id: string) => invoke<void>("unpin_item", { id });
export const deleteItem = (id: string) => invoke<void>("delete_item", { id });
export const clearUnpinned = () => invoke<void>("clear_unpinned");
export const copyItemToClipboard = (id: string) =>
  invoke<void>("copy_item_to_clipboard", { id });
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