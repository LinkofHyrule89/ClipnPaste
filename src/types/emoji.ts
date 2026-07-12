export type EmojiEntry = {
  char: string;
  name: string;
  category: string;
  keywords: string[];
  /** Hex codepoint id used for asset filenames, e.g. "1f600" */
  id: string;
  /** Which offline packs include this glyph (index v2). */
  packs?: {
    google?: boolean;
    fluent?: boolean;
  };
  /** Legacy index field (v1); unused when `id` is present. */
  image?: string;
};

export type EmojiIndex = {
  version: number;
  categories: string[];
  packs?: string[];
  emoji: EmojiEntry[];
};

export type ClipboardPanelTab = "history" | "emoji" | "gif";
