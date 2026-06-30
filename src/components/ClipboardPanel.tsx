import { useCallback, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  clearUnpinned,
  copyItemToClipboard,
  getHistory,
  pinItem,
  unpinItem,
} from "../api";
import type { ClipItemSummary } from "../types";

export function ClipboardPanel() {
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

  const close = async () => {
    await getCurrentWindow().hide();
  };

  const selectItem = async (item: ClipItemSummary) => {
    await copyItemToClipboard(item.id);
    await close();
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

  const handleClearAll = async () => {
    await clearUnpinned();
    await refresh();
  };

  useEffect(() => {
    const onKey = async (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        await close();
      }
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
    <div className="h-screen w-screen p-2">
      <div className="glass-panel flex h-full flex-col overflow-hidden text-white">
        <header className="flex items-center justify-between border-b border-white/10 px-4 py-3">
          <div>
            <h1 className="text-sm font-semibold">Clipboard</h1>
            <p className="text-xs text-white/50">Super+V</p>
          </div>
          <button
            onClick={() => void handleClearAll()}
            className="rounded-md px-3 py-1.5 text-xs text-white/80 hover:bg-white/10"
          >
            Clear all
          </button>
        </header>

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
            <button
              key={item.id}
              onClick={() => void selectItem(item)}
              className={`mb-2 flex w-full items-start gap-3 rounded-lg px-3 py-3 text-left transition ${
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
              <button
                onClick={(event) => void togglePin(item, event)}
                className={`rounded-md px-2 py-1 text-xs ${
                  item.pinned ? "text-sky-300" : "text-white/40 hover:text-white/70"
                }`}
                title={item.pinned ? "Unpin" : "Pin"}
              >
                {item.pinned ? "Pinned" : "Pin"}
              </button>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}