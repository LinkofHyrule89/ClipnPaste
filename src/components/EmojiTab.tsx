import { useEffect, useMemo, useRef, useState } from "react";
import { pasteTextToTarget } from "../api";
import {
  entryId,
  groupBySkinTone,
  loadSkinPrefs,
  resolvePreferred,
  saveSkinPref,
  type EmojiGroup,
} from "../emoji/groupVariants";
import type { EmojiEntry, EmojiIndex } from "../types/emoji";
import type { EmojiStyle } from "../types/settings";

type Props = {
  emojiStyle?: EmojiStyle;
};

const LONG_PRESS_MS = 420;

function assetUrl(style: EmojiStyle, entry: EmojiEntry, ext: "svg" | "png"): string {
  const id = entry.id || legacyIdFromImage(entry.image);
  return `/assets/emoji/${style}/${id}.${ext}`;
}

function legacyIdFromImage(image?: string): string {
  if (!image) return "";
  const base = image.split("/").pop() ?? "";
  return base.replace(/\.(svg|png)$/i, "");
}

function packHas(entry: EmojiEntry, style: "google" | "fluent"): boolean {
  if (entry.packs && entry.packs[style] === false) return false;
  if (entry.packs && entry.packs[style] === true) return true;
  return true;
}

function EmojiGlyph({
  entry,
  style,
  sizeClass = "h-8 w-8",
  textClass = "text-[1.65rem]",
}: {
  entry: EmojiEntry;
  style: EmojiStyle;
  sizeClass?: string;
  textClass?: string;
}) {
  const [failed, setFailed] = useState(false);
  const [ext, setExt] = useState<"svg" | "png">("svg");
  const [fallbackStyle, setFallbackStyle] = useState<"google" | "fluent" | null>(
    null,
  );

  useEffect(() => {
    setFailed(false);
    setExt("svg");
    setFallbackStyle(null);
  }, [entry.id, entry.char, style]);

  if (style === "system" || failed) {
    return (
      <span
        className={`${textClass} leading-none`}
        style={{
          fontFamily:
            '"Noto Color Emoji", "Apple Color Emoji", "Segoe UI Emoji", "Twemoji Mozilla", sans-serif',
        }}
      >
        {entry.char}
      </span>
    );
  }

  const active = fallbackStyle ?? style;
  const src = assetUrl(active, entry, ext);

  return (
    <img
      src={src}
      alt={entry.name}
      className={`${sizeClass} object-contain`}
      loading="lazy"
      draggable={false}
      onError={() => {
        if (ext === "svg") {
          setExt("png");
          return;
        }
        if (!fallbackStyle) {
          const other = active === "google" ? "fluent" : "google";
          if (packHas(entry, other)) {
            setFallbackStyle(other);
            setExt("svg");
            return;
          }
        }
        setFailed(true);
      }}
    />
  );
}

export function EmojiTab({ emojiStyle = "google" }: Props) {
  const [index, setIndex] = useState<EmojiIndex | null>(null);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("All");
  const [prefs, setPrefs] = useState<Record<string, string>>(() => loadSkinPrefs());
  const [openGroup, setOpenGroup] = useState<EmojiGroup | null>(null);
  const [popoverPos, setPopoverPos] = useState<{ top: number; left: number } | null>(
    null,
  );

  const longPressTimer = useRef<number | null>(null);
  const longPressFired = useRef(false);
  const pressStart = useRef<{
    t: number;
    x: number;
    y: number;
    group: EmojiGroup;
    el: HTMLElement;
  } | null>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const response = await fetch("/assets/emoji-index.json");
        if (!response.ok) throw new Error("emoji index missing");
        const data = (await response.json()) as EmojiIndex;
        data.emoji = data.emoji.map((entry) => {
          if (entry.id) return entry;
          return {
            ...entry,
            id: legacyIdFromImage(entry.image),
          };
        });
        if (!cancelled) setIndex(data);
      } catch {
        if (!cancelled) setIndex({ version: 2, categories: [], emoji: [] });
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const filtered = useMemo(() => {
    if (!index) return [];
    const q = query.trim().toLowerCase();
    return index.emoji.filter((entry) => {
      if (category !== "All" && entry.category !== category) return false;
      if (!q) return true;
      return (
        entry.name.toLowerCase().includes(q) ||
        entry.keywords.some((keyword) => keyword.includes(q))
      );
    });
  }, [index, query, category]);

  const groups = useMemo(() => groupBySkinTone(filtered), [filtered]);

  const categories = useMemo(() => {
    if (!index) return ["All"];
    return ["All", ...index.categories];
  }, [index]);

  const closePopover = () => {
    setOpenGroup(null);
    setPopoverPos(null);
  };

  useEffect(() => {
    if (!openGroup) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closePopover();
    };
    const onDown = (event: MouseEvent) => {
      const el = popoverRef.current;
      if (el && !el.contains(event.target as Node)) {
        closePopover();
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [openGroup]);

  const pickEmoji = async (entry: EmojiEntry, group?: EmojiGroup) => {
    // Paste first so prefs re-render cannot race hide/focus.
    try {
      await pasteTextToTarget(entry.char);
    } finally {
      if (group && group.variants.length > 1) {
        saveSkinPref(group.baseId, entryId(entry));
        setPrefs(loadSkinPrefs());
      }
      closePopover();
    }
  };

  const openVariants = (group: EmojiGroup, anchor: HTMLElement) => {
    if (group.variants.length < 2) return;
    const rect = anchor.getBoundingClientRect();
    const width = Math.min(280, window.innerWidth - 16);
    let left = rect.left + rect.width / 2 - width / 2;
    left = Math.max(8, Math.min(left, window.innerWidth - width - 8));
    const below = rect.bottom + 6;
    const top =
      below + 72 > window.innerHeight ? Math.max(8, rect.top - 78) : below;
    setPopoverPos({ top, left });
    setOpenGroup(group);
  };

  const clearLongPressTimer = () => {
    if (longPressTimer.current != null) {
      window.clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  };

  const onCellPointerDown = (
    event: React.PointerEvent<HTMLButtonElement>,
    group: EmojiGroup,
  ) => {
    if (event.button !== 0) return;
    longPressFired.current = false;
    clearLongPressTimer();
    pressStart.current = {
      t: Date.now(),
      x: event.clientX,
      y: event.clientY,
      group,
      el: event.currentTarget,
    };
    if (group.variants.length < 2) return;
    const target = event.currentTarget;
    longPressTimer.current = window.setTimeout(() => {
      longPressFired.current = true;
      pressStart.current = null;
      openVariants(group, target);
    }, LONG_PRESS_MS);
  };

  const onCellPointerUp = (event: React.PointerEvent<HTMLButtonElement>) => {
    if (event.button !== 0) return;
    clearLongPressTimer();
    const start = pressStart.current;
    pressStart.current = null;

    // Long-press already opened the sheet — do not paste.
    if (longPressFired.current) {
      longPressFired.current = false;
      return;
    }
    if (!start) return;

    const dt = Date.now() - start.t;
    const dist = Math.hypot(event.clientX - start.x, event.clientY - start.y);
    if (dt >= LONG_PRESS_MS || dist > 12) return;

    const entry = resolvePreferred(start.group, prefs);
    void pickEmoji(entry, start.group);
  };

  const onCellPointerCancel = () => {
    clearLongPressTimer();
    pressStart.current = null;
    longPressFired.current = false;
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="border-b border-white/10 px-3 py-2">
        <input
          type="search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search emoji"
          className="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm text-white placeholder:text-white/40 focus:border-sky-400/50 focus:outline-none"
          autoFocus
        />
      </div>

      <div className="flex gap-1 overflow-x-auto border-b border-white/10 px-3 pt-2 pb-3.5 [scrollbar-gutter:stable]">
        {categories.map((item) => (
          <button
            key={item}
            type="button"
            onClick={() => {
              setCategory(item);
              closePopover();
            }}
            className={`shrink-0 rounded-full px-3 py-1 text-xs transition ${
              category === item
                ? "bg-sky-500/30 text-sky-100"
                : "bg-white/5 text-white/60 hover:bg-white/10"
            }`}
          >
            {item}
          </button>
        ))}
      </div>

      <div className="relative flex-1 overflow-y-auto p-2 pt-3">
        {loading && (
          <p className="px-3 py-6 text-center text-sm text-white/50">Loading…</p>
        )}
        {!loading && groups.length === 0 && (
          <p className="px-3 py-6 text-center text-sm text-white/50">
            No emoji found.
          </p>
        )}
        <div className="grid grid-cols-8 gap-1">
          {groups.map((group) => {
            const display = resolvePreferred(group, prefs);
            const multi = group.variants.length > 1;
            return (
              <button
                key={group.baseId}
                type="button"
                title={
                  multi
                    ? `${display.name} (hold or right-click for skin tones)`
                    : display.name
                }
                onPointerDown={(e) => onCellPointerDown(e, group)}
                onPointerUp={onCellPointerUp}
                onPointerLeave={onCellPointerCancel}
                onPointerCancel={onCellPointerCancel}
                onContextMenu={(e) => {
                  if (!multi) return;
                  e.preventDefault();
                  clearLongPressTimer();
                  pressStart.current = null;
                  longPressFired.current = false;
                  openVariants(group, e.currentTarget);
                }}
                className="relative flex aspect-square items-center justify-center rounded-lg transition hover:bg-white/10"
              >
                <EmojiGlyph entry={display} style={emojiStyle} />
                {multi && (
                  <span
                    className="pointer-events-none absolute bottom-0.5 right-0.5 h-1.5 w-1.5 rounded-full bg-sky-400/90 shadow"
                    aria-hidden
                  />
                )}
              </button>
            );
          })}
        </div>
      </div>

      {openGroup && popoverPos && (
        <div
          ref={popoverRef}
          className="fixed z-[100] max-w-[min(280px,calc(100vw-16px))] rounded-xl border border-white/15 bg-neutral-900/98 p-2 shadow-2xl backdrop-blur-md"
          style={{ top: popoverPos.top, left: popoverPos.left, width: 280 }}
          role="dialog"
          aria-label="Skin tone variants"
        >
          <p className="mb-1.5 px-1 text-[10px] uppercase tracking-wide text-white/40">
            Skin tone
          </p>
          <div className="flex flex-wrap gap-1">
            {openGroup.variants.map((entry) => (
              <button
                key={entryId(entry)}
                type="button"
                title={entry.name}
                onClick={() => void pickEmoji(entry, openGroup)}
                className="flex h-10 w-10 items-center justify-center rounded-lg hover:bg-white/10"
              >
                <EmojiGlyph
                  entry={entry}
                  style={emojiStyle}
                  sizeClass="h-9 w-9"
                  textClass="text-[1.75rem]"
                />
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
