import { useCallback, useEffect, useState } from "react";
import {
  clearUnpinned,
  pasteItemToTarget,
  deleteItem,
  getHistory,
  pinItem,
  unpinItem,
} from "../api";
import type { ClipItemSummary } from "../types";

export function HistoryTab() {
  const [items, setItems] = useState<ClipItemSummary[]>([]);
  const [selected, setSelected] = useState(0);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const history = await getHistory();
      setItems(history);
      setSelected(0);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const selectItem = async (item: ClipItemSummary) => {
    await pasteItemToTarget(item.id);
  };

  const togglePin = async (item: ClipItemSummary, event: React.MouseEvent) => {
    event.stopPropagation();
    if (item.pinned) {
      await unpinItem(item.id);
    } else {
      await pinItem(item.id);
    }
    await refresh();
  };

  const removeItem = async (item: ClipItemSummary, event: React.MouseEvent) => {
    event.stopPropagation();
    await deleteItem(item.id);
    await refresh();
  };

  const handleClearAll = async () => {
    await clearUnpinned();
    await refresh();
  };

  useEffect(() => {
    const onKey = async (event: KeyboardEvent) => {
      if (event.key === "ArrowDown") {
        setSelected((value) => Math.min(value + 1, items.length - 1));
      }
      if (event.key === "ArrowUp") {
        setSelected((value) => Math.max(value - 1, 0));
      }
      if (event.key === "Enter" && items[selected]) {
        await selectItem(items[selected]);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [items, selected]);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center justify-end gap-1 border-b border-white/10 px-3 py-2">
        <button
          onClick={() => void handleClearAll()}
          className="rounded-md px-3 py-1.5 text-xs text-white/80 hover:bg-white/10"
        >
          Clear all
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {loading && (
          <p className="px-3 py-6 text-center text-sm text-white/50">Loading…</p>
        )}
        {!loading && items.length === 0 && (
          <p className="px-3 py-6 text-center text-sm text-white/50">
            Copy something to get started.
          </p>
        )}
        {items.map((item, index) => (
          <div
            key={item.id}
            role="button"
            tabIndex={0}
            onClick={() => void selectItem(item)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                void selectItem(item);
              }
            }}
            className={`mb-2 flex w-full cursor-pointer items-start gap-3 rounded-lg px-3 py-3 text-left transition ${
              index === selected ? "bg-sky-500/20" : "hover:bg-white/5"
            }`}
          >
            <div className="min-w-0 flex-1">
              {item.itemType === "image" ? (
                <img
                  src={item.preview}
                  alt="Clipboard image"
                  className="max-h-24 rounded-md border border-white/10 object-contain"
                />
              ) : (
                <p className="line-clamp-3 whitespace-pre-wrap text-sm text-white/90">
                  {item.preview}
                </p>
              )}
            </div>
            <div className="flex shrink-0 items-center gap-1 self-start">
              <button
                type="button"
                onClick={(event) => void removeItem(item, event)}
                className="clipboard-action-btn text-white/45 hover:bg-red-500/20 hover:text-red-300"
                title="Delete"
                aria-label="Delete item"
              >
                🗑
              </button>
              <button
                type="button"
                onClick={(event) => void togglePin(item, event)}
                className={`clipboard-action-btn ${
                  item.pinned ? "text-sky-300" : "text-white/40 hover:text-white/70"
                }`}
                title={item.pinned ? "Unpin" : "Pin"}
                aria-label={item.pinned ? "Unpin item" : "Pin item"}
              >
                {item.pinned ? "📌" : "📍"}
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}