export type ClipItemType = "text" | "image";

export interface ClipItemSummary {
  id: string;
  itemType: ClipItemType;
  preview: string;
  pinned: boolean;
  createdAt: number;
}

export interface CaptureResult {
  pngBase64: string;
  width: number;
  height: number;
  /** Absolute path under Screenshots when auto-saved. */
  savedPath?: string | null;
}

export interface WindowInfo {
  id: number;
  title: string;
  appName: string;
}