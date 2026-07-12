import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import type { EmojiEntry } from "../types/emoji";
import {
  baseId,
  entryId,
  groupBySkinTone,
  hasSkinTone,
  pickDefault,
  resolvePreferred,
  sortVariants,
} from "./groupVariants";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "../..");

function loadIndexEmoji(): EmojiEntry[] {
  const raw = JSON.parse(
    readFileSync(join(ROOT, "public/assets/emoji-index.json"), "utf8"),
  ) as { emoji: EmojiEntry[] };
  return raw.emoji;
}

function codepoints(s: string): string[] {
  return [...s].map((c) => (c.codePointAt(0) ?? 0).toString(16));
}

const THUMBS = {
  default: "\u{1F44D}",
  light: "\u{1F44D}\u{1F3FB}",
  mediumLight: "\u{1F44D}\u{1F3FC}",
  medium: "\u{1F44D}\u{1F3FD}",
  mediumDark: "\u{1F44D}\u{1F3FE}",
  dark: "\u{1F44D}\u{1F3FF}",
} as const;

function makeThumbsEntries(): EmojiEntry[] {
  return [
    { char: THUMBS.default, name: "thumbs up", category: "People", keywords: [], id: "1f44d" },
    {
      char: THUMBS.light,
      name: "thumbs up: light skin tone",
      category: "People",
      keywords: [],
      id: "1f44d-1f3fb",
    },
    {
      char: THUMBS.dark,
      name: "thumbs up: dark skin tone",
      category: "People",
      keywords: [],
      id: "1f44d-1f3ff",
    },
    {
      char: THUMBS.medium,
      name: "thumbs up: medium skin tone",
      category: "People",
      keywords: [],
      id: "1f44d-1f3fd",
    },
  ];
}

describe("skin tone Unicode (what paste must send)", () => {
  it("dark thumbs-up is base + Fitzpatrick dark modifier, not yellow alone", () => {
    expect(codepoints(THUMBS.default)).toEqual(["1f44d"]);
    expect(codepoints(THUMBS.dark)).toEqual(["1f44d", "1f3ff"]);
    expect(THUMBS.dark).not.toBe(THUMBS.default);
    expect(THUMBS.dark.length).toBeGreaterThan(THUMBS.default.length);
  });

  it("shipped emoji-index stores multi-codepoint skin-tone chars correctly", () => {
    const all = loadIndexEmoji();
    const dark = all.find((e) => e.id === "1f44d-1f3ff" || e.name === "thumbs up: dark skin tone");
    const base = all.find((e) => e.id === "1f44d" || e.name === "thumbs up");
    expect(base, "base thumbs up in index").toBeTruthy();
    expect(dark, "dark thumbs up in index").toBeTruthy();
    expect(codepoints(base!.char)).toEqual(["1f44d"]);
    expect(codepoints(dark!.char)).toEqual(["1f44d", "1f3ff"]);
    // Pasting dark must not equal the yellow/default glyph string
    expect(dark!.char).not.toEqual(base!.char);
  });
});

describe("groupBySkinTone", () => {
  it("collapses thumbs-up variants into one group with default yellow first", () => {
    const groups = groupBySkinTone(makeThumbsEntries());
    expect(groups).toHaveLength(1);
    const g = groups[0]!;
    expect(g.baseId).toBe("1f44d");
    expect(g.variants.length).toBe(4);
    expect(hasSkinTone(g.defaultEntry)).toBe(false);
    expect(g.defaultEntry.char).toBe(THUMBS.default);
    expect(g.variants.map((v) => v.id)).toEqual([
      "1f44d",
      "1f44d-1f3fb",
      "1f44d-1f3fd",
      "1f44d-1f3ff",
    ]);
  });

  it("does not merge different base emoji", () => {
    const entries: EmojiEntry[] = [
      ...makeThumbsEntries(),
      {
        char: "\u{1F44E}",
        name: "thumbs down",
        category: "People",
        keywords: [],
        id: "1f44e",
      },
    ];
    const groups = groupBySkinTone(entries);
    expect(groups.map((g) => g.baseId).sort()).toEqual(["1f44d", "1f44e"]);
  });
});

describe("resolvePreferred (click paste selection)", () => {
  it("without prefs returns yellow/default", () => {
    const group = groupBySkinTone(makeThumbsEntries())[0]!;
    const entry = resolvePreferred(group, {});
    expect(entry.char).toBe(THUMBS.default);
    expect(codepoints(entry.char)).toEqual(["1f44d"]);
  });

  it("with dark tone pref returns the multi-codepoint dark char for paste", () => {
    const group = groupBySkinTone(makeThumbsEntries())[0]!;
    const entry = resolvePreferred(group, { [group.baseId]: "1f44d-1f3ff" });
    expect(entryId(entry)).toBe("1f44d-1f3ff");
    expect(entry.char).toBe(THUMBS.dark);
    expect(codepoints(entry.char)).toEqual(["1f44d", "1f3ff"]);
    // Simulated "clipboard payload" is exactly what paste_text_to_target receives
    const clipboardPayload = entry.char;
    expect(clipboardPayload).not.toBe(THUMBS.default);
    expect([...clipboardPayload].map((c) => c.codePointAt(0))).toEqual([
      0x1f44d, 0x1f3ff,
    ]);
  });

  it("ignores stale prefs and falls back to default", () => {
    const group = groupBySkinTone(makeThumbsEntries())[0]!;
    const entry = resolvePreferred(group, { [group.baseId]: "dead-beef" });
    expect(entry.char).toBe(THUMBS.default);
  });
});

describe("baseId / sorting helpers", () => {
  it("baseId strips skin tones", () => {
    const dark = makeThumbsEntries().find((e) => e.id === "1f44d-1f3ff")!;
    expect(baseId(dark)).toBe("1f44d");
  });

  it("sortVariants orders default then light-to-dark", () => {
    const sorted = sortVariants(makeThumbsEntries());
    expect(sorted.map((e) => e.id)).toEqual([
      "1f44d",
      "1f44d-1f3fb",
      "1f44d-1f3fd",
      "1f44d-1f3ff",
    ]);
    expect(pickDefault(sorted).id).toBe("1f44d");
  });
});

describe("index-wide skin tone integrity sample", () => {
  it("every skin-tone variant char includes a Fitzpatrick modifier codepoint", () => {
    const tones = new Set([0x1f3fb, 0x1f3fc, 0x1f3fd, 0x1f3fe, 0x1f3ff]);
    const all = loadIndexEmoji();
    const withTone = all.filter((e) =>
      e.id.split("-").some((p) => ["1f3fb", "1f3fc", "1f3fd", "1f3fe", "1f3ff"].includes(p)),
    );
    expect(withTone.length).toBeGreaterThan(100);
    for (const e of withTone) {
      const cps = [...e.char].map((c) => c.codePointAt(0)!);
      expect(
        cps.some((cp) => tones.has(cp)),
        `${e.name} (${e.id}) char=${JSON.stringify(e.char)} cps=${cps.map((c) => c.toString(16))}`,
      ).toBe(true);
    }
  });
});
