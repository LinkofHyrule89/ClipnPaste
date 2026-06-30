import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getSettings, showSettings } from "../api";
import type { AppSettings } from "../types/settings";
import { EmojiTab } from "./EmojiTab";
import { GifTab } from "./GifTab";
import { HistoryTab } from "./HistoryTab";
import type { ClipboardPanelTab } from "../types/emoji";

function tabFromQuery(): ClipboardPanelTab {
  const params = new URLSearchParams(window.location.search);
  const tab = params.get("tab");
  if (tab === "emoji" || tab === "gif") return tab;
  return "history";
}

const ALL_TABS: { id: ClipboardPanelTab; label: string; hint?: string }[] = [
  { id: "history", label: "History", hint: "Super+V" },
  { id: "emoji", label: "Emoji", hint: "Super+;" },
  { id: "gif", label: "GIF" },
];

function isTabVisible(tab: ClipboardPanelTab, settings: AppSettings) {
  if (tab === "emoji") return settings.emojiTabEnabled;
  if (tab === "gif") return settings.gifTabEnabled;
  return true;
}

export function ClipboardPanel() {
  const [tab, setTab] = useState<ClipboardPanelTab>(tabFromQuery);
  const [settings, setSettings] = useState<AppSettings>({
    emojiTabEnabled: true,
    gifTabEnabled: true,
  });
  const focusReady = useRef(false);

  const visibleTabs = useMemo(
    () => ALL_TABS.filter((item) => isTabVisible(item.id, settings)),
    [settings],
  );

  const refreshSettings = useCallback(async () => {
    const current = await getSettings();
    setSettings(current);
    setTab((active) => (isTabVisible(active, current) ? active : "history"));
  }, []);

  useEffect(() => {
    void refreshSettings();
  }, [refreshSettings]);

  useEffect(() => {
    const panel = getCurrentWindow();
    let unlistenFocus: (() => void) | undefined;
    let unlistenTab: (() => void) | undefined;
    let unlistenSettings: (() => void) | undefined;
    const readyTimer = setTimeout(() => {
      focusReady.current = true;
    }, 200);

    void (async () => {
      await panel.setAlwaysOnTop(true);
      unlistenFocus = await panel.onFocusChanged(({ payload: focused }) => {
        if (focusReady.current && !focused) {
          void panel.hide();
        }
      });
      unlistenTab = await listen<string>("set-clipboard-tab", (event) => {
        const next = event.payload;
        if (next === "history") {
          setTab("history");
        } else if (next === "emoji" && settings.emojiTabEnabled) {
          setTab("emoji");
        } else {
          setTab("history");
        }
      });
      unlistenSettings = await listen<AppSettings>("settings-changed", (event) => {
        const next = event.payload;
        setSettings(next);
        setTab((active) => (isTabVisible(active, next) ? active : "history"));
      });
    })();

    return () => {
      clearTimeout(readyTimer);
      focusReady.current = false;
      unlistenFocus?.();
      unlistenTab?.();
      unlistenSettings?.();
    };
  }, [settings.emojiTabEnabled]);

  const close = async () => {
    await getCurrentWindow().hide();
  };

  const openSettings = async () => {
    await showSettings();
  };

  const startDrag = (event: React.MouseEvent) => {
    if (event.button === 0) {
      void getCurrentWindow().startDragging();
    }
  };

  useEffect(() => {
    const onKey = async (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        await close();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const activeTab = ALL_TABS.find((item) => item.id === tab);
  const showTabBar = visibleTabs.length > 1;

  return (
    <div className="h-screen w-screen p-2">
      <div className="glass-panel flex h-full flex-col overflow-hidden text-white">
        <header className="flex items-center justify-between border-b border-white/10 px-4 py-3">
          <div
            className="clipboard-drag-handle min-w-0 flex-1 select-none"
            onMouseDown={startDrag}
            title="Drag to move"
          >
            <h1 className="text-sm font-semibold">Clipboard</h1>
            <p className="text-xs text-white/50">{activeTab?.hint ?? "ClipnPaste"}</p>
          </div>
          <div className="flex items-center gap-1">
            <button
              onClick={() => void openSettings()}
              className="flex h-8 w-8 items-center justify-center rounded-md text-base text-white/70 hover:bg-white/10 hover:text-white"
              title="Settings"
              aria-label="Open settings"
            >
              ⚙
            </button>
            <button
              onClick={() => void close()}
              className="flex h-8 w-8 items-center justify-center rounded-md text-lg text-white/70 hover:bg-white/10 hover:text-white"
              title="Close"
              aria-label="Close clipboard"
            >
              ×
            </button>
          </div>
        </header>

        {showTabBar && (
          <nav className="flex border-b border-white/10 px-2 pt-1">
            {visibleTabs.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => setTab(item.id)}
                className={`relative px-4 py-2 text-xs font-medium transition ${
                  tab === item.id
                    ? "text-sky-200"
                    : "text-white/50 hover:text-white/80"
                }`}
              >
                {item.label}
                {tab === item.id && (
                  <span className="absolute inset-x-2 bottom-0 h-0.5 rounded-full bg-sky-400" />
                )}
              </button>
            ))}
          </nav>
        )}

        <div className="flex min-h-0 flex-1 flex-col">
          {tab === "history" && <HistoryTab />}
          {tab === "emoji" && settings.emojiTabEnabled && <EmojiTab />}
          {tab === "gif" && settings.gifTabEnabled && <GifTab />}
        </div>
      </div>
    </div>
  );
}