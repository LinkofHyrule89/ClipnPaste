export type EmojiEntry = {
  char: string;
  name: string;
  category: string;
  keywords: string[];
  image: string;
};

export type EmojiIndex = {
  version: number;
  categories: string[];
  emoji: EmojiEntry[];
};

export type ClipboardPanelTab = "history" | "emoji" | "gif";