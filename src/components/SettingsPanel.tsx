import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getSettings, openKeyboardShortcuts, setSettings } from "../api";
import type { AppSettings } from "../types/settings";

export function SettingsPanel() {
  const [settings, setLocalSettings] = useState<AppSettings>({
    emojiTabEnabled: true,
    gifTabEnabled: true,
  });
  const [loading, setLoading] = useState(true);
  const [shortcutsError, setShortcutsError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const current = await getSettings();
        setLocalSettings(current);
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  useEffect(() => {
    void getCurrentWindow().setAlwaysOnTop(true);
  }, []);

  const close = async () => {
    await getCurrentWindow().hide();
  };

  const startDrag = (event: React.MouseEvent) => {
    if (event.button === 0) {
      void getCurrentWindow().startDragging();
    }
  };

  const updateSetting = async (patch: Partial<AppSettings>) => {
    const next = { ...settings, ...patch };
    setLocalSettings(next);
    await setSettings(next);
  };

  const handleOpenShortcuts = async () => {
    setShortcutsError(null);
    try {
      await openKeyboardShortcuts();
    } catch (error) {
      setShortcutsError(
        error instanceof Error ? error.message : "Failed to open keyboard settings",
      );
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

  return (
    <div className="h-screen w-screen p-2">
      <div className="glass-panel flex h-full flex-col overflow-hidden text-white">
        <header className="flex items-center justify-between border-b border-white/10 px-4 py-3">
          <div
            className="clipboard-drag-handle min-w-0 flex-1 select-none"
            onMouseDown={startDrag}
            title="Drag to move"
          >
            <h1 className="text-sm font-semibold">Settings</h1>
            <p className="text-xs text-white/50">ClipnPaste</p>
          </div>
          <button
            onClick={() => void close()}
            className="flex h-8 w-8 items-center justify-center rounded-md text-lg text-white/70 hover:bg-white/10 hover:text-white"
            title="Close"
            aria-label="Close settings"
          >
            ×
          </button>
        </header>

        <div className="flex-1 overflow-y-auto p-4">
          {loading ? (
            <p className="text-sm text-white/50">Loading…</p>
          ) : (
            <div className="space-y-5">
              <section>
                <h2 className="mb-3 text-xs font-medium uppercase tracking-wide text-white/50">
                  Clipboard tabs
                </h2>
                <label className="mb-3 flex cursor-pointer items-center justify-between rounded-lg bg-white/5 px-3 py-3">
                  <span className="text-sm text-white/90">Show Emoji tab</span>
                  <input
                    type="checkbox"
                    checked={settings.emojiTabEnabled}
                    onChange={(event) =>
                      void updateSetting({ emojiTabEnabled: event.target.checked })
                    }
                    className="h-4 w-4 accent-sky-400"
                  />
                </label>
                <label className="flex cursor-pointer items-center justify-between rounded-lg bg-white/5 px-3 py-3">
                  <span className="text-sm text-white/90">Show GIF tab</span>
                  <input
                    type="checkbox"
                    checked={settings.gifTabEnabled}
                    onChange={(event) =>
                      void updateSetting({ gifTabEnabled: event.target.checked })
                    }
                    className="h-4 w-4 accent-sky-400"
                  />
                </label>
              </section>

              <section>
                <h2 className="mb-3 text-xs font-medium uppercase tracking-wide text-white/50">
                  Shortcuts
                </h2>
                <button
                  type="button"
                  onClick={() => void handleOpenShortcuts()}
                  className="w-full rounded-lg bg-white/5 px-3 py-3 text-left text-sm text-sky-200 transition hover:bg-white/10"
                >
                  Keyboard shortcuts…
                </button>
                <p className="mt-2 text-xs text-white/45">
                  Opens system keyboard settings to change ClipnPaste hotkeys.
                </p>
                {shortcutsError && (
                  <p className="mt-2 text-xs text-red-300">{shortcutsError}</p>
                )}
              </section>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}