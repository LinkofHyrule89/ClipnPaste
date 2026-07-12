import type { EmojiEntry } from "../types/emoji";

/** Fitzpatrick skin-tone modifiers */
export const SKIN_TONES = new Set([
  "1f3fb",
  "1f3fc",
  "1f3fd",
  "1f3fe",
  "1f3ff",
]);

const TONE_ORDER = ["1f3fb", "1f3fc", "1f3fd", "1f3fe", "1f3ff"] as const;

export type EmojiGroup = {
  baseId: string;
  /** Preferred display / default paste (usually no skin tone). */
  defaultEntry: EmojiEntry;
  variants: EmojiEntry[];
};

function entryId(entry: EmojiEntry): string {
  if (entry.id) return entry.id.toLowerCase();
  if (entry.image) {
    const base = entry.image.split("/").pop() ?? "";
    return base.replace(/\.(svg|png)$/i, "").toLowerCase();
  }
  return [...entry.char]
    .map((c) => (c.codePointAt(0) ?? 0).toString(16))
    .join("-");
}

export function partsOf(entry: EmojiEntry): string[] {
  return entryId(entry).split("-").filter(Boolean);
}

export function hasSkinTone(entry: EmojiEntry): boolean {
  return partsOf(entry).some((p) => SKIN_TONES.has(p));
}

/** Strip skin tones and emoji presentation selectors for grouping. */
export function baseId(entry: EmojiEntry): string {
  const parts = partsOf(entry).filter((p) => p !== "fe0f" && !SKIN_TONES.has(p));
  return parts.join("-") || entryId(entry);
}

function toneSortKey(entry: EmojiEntry): [number, string] {
  const tones = partsOf(entry).filter((p) => SKIN_TONES.has(p));
  if (tones.length === 0) return [-1, ""];
  if (tones.length === 1) {
    const i = TONE_ORDER.indexOf(tones[0] as (typeof TONE_ORDER)[number]);
    return [i >= 0 ? i : 50, ""];
  }
  // Multi-person dual tones: after single-tone variants
  const ranks = tones
    .map((t) => TONE_ORDER.indexOf(t as (typeof TONE_ORDER)[number]))
    .join("-");
  return [100, ranks];
}

export function sortVariants(entries: EmojiEntry[]): EmojiEntry[] {
  return [...entries].sort((a, b) => {
    const [ra, sa] = toneSortKey(a);
    const [rb, sb] = toneSortKey(b);
    if (ra !== rb) return ra - rb;
    if (sa !== sb) return sa.localeCompare(sb);
    return entryId(a).localeCompare(entryId(b));
  });
}

export function pickDefault(entries: EmojiEntry[]): EmojiEntry {
  const noTone = entries.find((e) => !hasSkinTone(e));
  return noTone ?? sortVariants(entries)[0]!;
}

/** Group a flat list of emoji by skin-tone base (preserves first-seen order of bases). */
export function groupBySkinTone(entries: EmojiEntry[]): EmojiGroup[] {
  const map = new Map<string, EmojiEntry[]>();
  const order: string[] = [];

  for (const entry of entries) {
    const key = baseId(entry);
    if (!map.has(key)) {
      map.set(key, []);
      order.push(key);
    }
    map.get(key)!.push(entry);
  }

  return order.map((key) => {
    const variants = sortVariants(map.get(key)!);
    return {
      baseId: key,
      defaultEntry: pickDefault(variants),
      variants,
    };
  });
}

const PREFS_KEY = "clipnpaste.emojiSkinPrefs";

export function loadSkinPrefs(): Record<string, string> {
  try {
    const raw = localStorage.getItem(PREFS_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, string>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

export function saveSkinPref(base: string, emojiId: string) {
  const prefs = loadSkinPrefs();
  prefs[base] = emojiId;
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
  } catch {
    // ignore quota
  }
}

export function resolvePreferred(
  group: EmojiGroup,
  prefs: Record<string, string>,
): EmojiEntry {
  const want = prefs[group.baseId];
  if (want) {
    const hit = group.variants.find((e) => entryId(e) === want.toLowerCase());
    if (hit) return hit;
  }
  return group.defaultEntry;
}

export { entryId };
