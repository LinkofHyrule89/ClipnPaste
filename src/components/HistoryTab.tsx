import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  clearUnpinned,
  pasteItemToTarget,
  deleteItem,
  getHistory,
  getItemContent,
  pinItem,
  unpinItem,
  updateItemText,
} from "../api";
import type { ClipItemSummary } from "../types";

const HISTORY_CHANGED = "history-changed";

export function HistoryTab() {
  const [items, setItems] = useState<ClipItemSummary[]>([]);
  const [selected, setSelected] = useState(0);
  const [loading, setLoading] = useState(true);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editDraft, setEditDraft] = useState("");
  const [editLoading, setEditLoading] = useState(false);
  const [editSaving, setEditSaving] = useState(false);
  const hasLoaded = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const refresh = useCallback(async (opts?: { showLoading?: boolean }) => {
    const showLoading = opts?.showLoading ?? !hasLoaded.current;
    if (showLoading) {
      setLoading(true);
    }
    try {
      const history = await getHistory();
      setItems(history);
      if (!hasLoaded.current) {
        setSelected(0);
      } else {
        setSelected((prev) => {
          if (history.length === 0) return 0;
          return Math.min(prev, history.length - 1);
        });
      }
      hasLoaded.current = true;
    } finally {
      if (showLoading) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void refresh({ showLoading: true });
  }, [refresh]);

  useEffect(() => {
    let unlistenEvent: (() => void) | undefined;
    let unlistenFocus: (() => void) | undefined;

    void (async () => {
      unlistenEvent = await listen(HISTORY_CHANGED, () => {
        if (editingId) return;
        void refresh({ showLoading: false });
      });

      const panel = getCurrentWindow();
      unlistenFocus = await panel.onFocusChanged(({ payload: focused }) => {
        if (focused && !editingId) {
          void refresh({ showLoading: false });
        }
      });
    })();

    return () => {
      unlistenEvent?.();
      unlistenFocus?.();
    };
  }, [refresh, editingId]);

  useEffect(() => {
    if (editingId && textareaRef.current) {
      textareaRef.current.focus();
      textareaRef.current.select();
    }
  }, [editingId, editLoading]);

  const selectItem = async (item: ClipItemSummary) => {
    if (editingId) return;
    await pasteItemToTarget(item.id);
  };

  const startEdit = async (item: ClipItemSummary, event: React.MouseEvent) => {
    event.stopPropagation();
    if (item.itemType !== "text") return;
    setEditingId(item.id);
    setEditDraft("");
    setEditLoading(true);
    try {
      const content = await getItemContent(item.id);
      setEditDraft(content);
    } catch {
      setEditingId(null);
    } finally {
      setEditLoading(false);
    }
  };

  const cancelEdit = (event?: React.MouseEvent) => {
    event?.stopPropagation();
    setEditingId(null);
    setEditDraft("");
    setEditSaving(false);
  };

  const saveEdit = async (item: ClipItemSummary, event: React.MouseEvent) => {
    event.stopPropagation();
    if (editSaving) return;
    setEditSaving(true);
    try {
      await updateItemText(item.id, editDraft);
      setEditingId(null);
      setEditDraft("");
      await refresh({ showLoading: false });
    } finally {
      setEditSaving(false);
    }
  };

  const togglePin = async (item: ClipItemSummary, event: React.MouseEvent) => {
    event.stopPropagation();
    if (item.pinned) {
      await unpinItem(item.id);
    } else {
      await pinItem(item.id);
    }
    await refresh({ showLoading: false });
  };

  const removeItem = async (item: ClipItemSummary, event: React.MouseEvent) => {
    event.stopPropagation();
    if (editingId === item.id) {
      cancelEdit();
    }
    await deleteItem(item.id);
    await refresh({ showLoading: false });
  };

  const handleClearAll = async () => {
    cancelEdit();
    await clearUnpinned();
    await refresh({ showLoading: false });
  };

  useEffect(() => {
    const onKey = async (event: KeyboardEvent) => {
      if (editingId) {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          cancelEdit();
        }
        return;
      }
      if (event.key === "ArrowDown") {
        setSelected((value) => Math.min(value + 1, Math.max(items.length - 1, 0)));
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
  }, [items, selected, editingId, editDraft]);

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
        {items.map((item, index) => {
          const isEditing = editingId === item.id;
          return (
            <div
              key={item.id}
              role={isEditing ? "group" : "button"}
              tabIndex={isEditing ? -1 : 0}
              onClick={() => {
                if (!isEditing) void selectItem(item);
              }}
              onKeyDown={(event) => {
                if (isEditing) return;
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  void selectItem(item);
                }
              }}
              className={`mb-2 flex w-full items-start gap-3 rounded-lg px-3 py-3 text-left transition ${
                isEditing
                  ? "cursor-default bg-sky-500/15 ring-1 ring-sky-400/40"
                  : `cursor-pointer ${
                      index === selected ? "bg-sky-500/20" : "hover:bg-white/5"
                    }`
              }`}
            >
              <div className="min-w-0 flex-1">
                {isEditing ? (
                  <div
                    className="flex flex-col gap-2"
                    onClick={(event) => event.stopPropagation()}
                  >
                    {editLoading ? (
                      <p className="text-sm text-white/50">Loading…</p>
                    ) : (
                      <textarea
                        ref={textareaRef}
                        value={editDraft}
                        onChange={(event) => setEditDraft(event.target.value)}
                        rows={6}
                        className="w-full resize-y rounded-md border border-white/15 bg-black/40 px-3 py-2 text-sm text-white/90 outline-none focus:border-sky-400/60"
                        placeholder="Edit clipboard text…"
                        disabled={editSaving}
                      />
                    )}
                    <div className="flex items-center justify-end gap-2">
                      <button
                        type="button"
                        onClick={(event) => cancelEdit(event)}
                        disabled={editSaving}
                        className="rounded-md px-3 py-1.5 text-xs text-white/70 hover:bg-white/10"
                      >
                        Cancel
                      </button>
                      <button
                        type="button"
                        onClick={(event) => void saveEdit(item, event)}
                        disabled={editSaving || editLoading}
                        className="rounded-md bg-sky-500/30 px-3 py-1.5 text-xs font-medium text-sky-100 hover:bg-sky-500/45 disabled:opacity-50"
                      >
                        {editSaving ? "Saving…" : "Save"}
                      </button>
                    </div>
                  </div>
                ) : item.itemType === "image" ? (
                  <img
                    src={item.preview}
                    alt="Clipboard image"
                    className="max-h-24 rounded-md border border-white/10 object-contain"
                  />
                ) : (
                  <p className="min-w-0 max-h-32 overflow-y-auto whitespace-pre-wrap break-words text-sm text-white/90">
                    {item.preview}
                  </p>
                )}
              </div>
              {!isEditing && (
                <div className="flex shrink-0 items-center gap-1 self-start">
                  {item.itemType === "text" && (
                    <button
                      type="button"
                      onClick={(event) => void startEdit(item, event)}
                      className="clipboard-action-btn text-white/45 hover:text-sky-200"
                      title="Edit"
                      aria-label="Edit item"
                    >
                      ✎
                    </button>
                  )}
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
                      item.pinned
                        ? "text-sky-300"
                        : "text-white/40 hover:text-white/70"
                    }`}
                    title={item.pinned ? "Unpin" : "Pin"}
                    aria-label={item.pinned ? "Unpin item" : "Pin item"}
                  >
                    {item.pinned ? "📌" : "📍"}
                  </button>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
