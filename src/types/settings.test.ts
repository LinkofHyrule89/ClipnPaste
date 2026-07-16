import { describe, expect, it } from "vitest";
import { DEFAULT_SETTINGS, type EmojiStyle } from "./settings";

describe("DEFAULT_SETTINGS", () => {
  it("enables emoji and gif tabs by default", () => {
    expect(DEFAULT_SETTINGS.emojiTabEnabled).toBe(true);
    expect(DEFAULT_SETTINGS.gifTabEnabled).toBe(true);
  });

  it("defaults emoji style to google", () => {
    expect(DEFAULT_SETTINGS.emojiStyle).toBe("google");
  });

  it("emoji styles are the known set", () => {
    const styles: EmojiStyle[] = ["google", "fluent", "system"];
    expect(styles).toContain(DEFAULT_SETTINGS.emojiStyle);
  });
});
