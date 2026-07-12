export type EmojiStyle = "google" | "fluent" | "system";

export type AppSettings = {
  emojiTabEnabled: boolean;
  gifTabEnabled: boolean;
  /** Offline art pack for the emoji picker. Default: google (Noto). */
  emojiStyle: EmojiStyle;
};

export const DEFAULT_SETTINGS: AppSettings = {
  emojiTabEnabled: true,
  gifTabEnabled: true,
  emojiStyle: "google",
};
