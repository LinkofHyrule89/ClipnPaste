import { useEffect, useMemo, useState } from "react";
import { pasteTextToTarget } from "../api";
import type { EmojiEntry, EmojiIndex } from "../types/emoji";

export function EmojiTab() {
  const [index, setIndex] = useState<EmojiIndex | null>(null);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState("All");

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const response = await fetch("/assets/emoji-index.json");
        if (!response.ok) throw new Error("emoji index missing");
        const data = (await response.json()) as EmojiIndex;
        if (!cancelled) setIndex(data);
      } catch {
        if (!cancelled) setIndex({ version: 1, categories: [], emoji: [] });
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

  const categories = useMemo(() => {
    if (!index) return ["All"];
    return ["All", ...index.categories];
  }, [index]);

  const pickEmoji = async (entry: EmojiEntry) => {
    await pasteTextToTarget(entry.char);
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

      <div className="flex gap-1 overflow-x-auto border-b border-white/10 px-3 py-2">
        {categories.map((item) => (
          <button
            key={item}
            type="button"
            onClick={() => setCategory(item)}
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

      <div className="flex-1 overflow-y-auto p-2">
        {loading && (
          <p className="px-3 py-6 text-center text-sm text-white/50">Loading…</p>
        )}
        {!loading && filtered.length === 0 && (
          <p className="px-3 py-6 text-center text-sm text-white/50">
            No emoji found.
          </p>
        )}
        <div className="grid grid-cols-8 gap-1">
          {filtered.map((entry) => (
            <button
              key={`${entry.char}-${entry.name}`}
              type="button"
              title={entry.name}
              onClick={() => void pickEmoji(entry)}
              className="flex aspect-square items-center justify-center rounded-lg transition hover:bg-white/10"
            >
              <img
                src={entry.image}
                alt={entry.name}
                className="h-8 w-8 object-contain"
                loading="lazy"
              />
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}