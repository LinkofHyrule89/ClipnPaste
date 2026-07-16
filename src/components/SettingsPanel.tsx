import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  getAppVersion,
  getSettings,
  openKeyboardShortcuts,
  openProjectPage,
  setSettings,
} from "../api";
import type { AppSettings, EmojiStyle } from "../types/settings";
import { DEFAULT_SETTINGS } from "../types/settings";

const EMOJI_STYLES: { id: EmojiStyle; label: string; hint: string }[] = [
  { id: "google", label: "Google", hint: "Noto Color Emoji" },
  { id: "fluent", label: "Fluent", hint: "Microsoft Fluent UI" },
  { id: "system", label: "System", hint: "Your OS emoji font" },
];

const GITHUB_URL = "https://github.com/LinkofHyrule89/ClipnPaste";

export function SettingsPanel() {
  const [settings, setLocalSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [version, setVersion] = useState<string | null>(null);
  const [shortcutsError, setShortcutsError] = useState<string | null>(null);
  const [linkError, setLinkError] = useState<string | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        const [current, appVersion] = await Promise.all([
          getSettings(),
          getAppVersion().catch(() => null),
        ]);
        setLocalSettings(current);
        if (appVersion) setVersion(appVersion);
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

  const handleOpenGitHub = async () => {
    setLinkError(null);
    try {
      await openProjectPage();
    } catch (error) {
      setLinkError(
        error instanceof Error ? error.message : "Failed to open GitHub page",
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
            <p className="text-xs text-white/50">
              ClipnPaste{version ? ` · v${version}` : ""}
            </p>
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
                  Emoji style
                </h2>
                <div className="space-y-2">
                  {EMOJI_STYLES.map((item) => {
                    const active = (settings.emojiStyle ?? "google") === item.id;
                    return (
                      <button
                        key={item.id}
                        type="button"
                        onClick={() => void updateSetting({ emojiStyle: item.id })}
                        className={`flex w-full items-center justify-between rounded-lg px-3 py-3 text-left transition ${
                          active
                            ? "bg-sky-500/25 ring-1 ring-sky-400/40"
                            : "bg-white/5 hover:bg-white/10"
                        }`}
                      >
                        <span>
                          <span className="block text-sm text-white/90">{item.label}</span>
                          <span className="block text-xs text-white/45">{item.hint}</span>
                        </span>
                        <span
                          className={`flex h-4 w-4 items-center justify-center rounded-full border ${
                            active ? "border-sky-300 bg-sky-400" : "border-white/30"
                          }`}
                          aria-hidden
                        >
                          {active && (
                            <span className="h-1.5 w-1.5 rounded-full bg-neutral-950" />
                          )}
                        </span>
                      </button>
                    );
                  })}
                </div>
                <p className="mt-2 text-xs text-white/45">
                  Offline Google and Fluent art ship with the app. System uses your desktop emoji font.
                </p>
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

              <section>
                <h2 className="mb-3 text-xs font-medium uppercase tracking-wide text-white/50">
                  About
                </h2>
                <div className="rounded-lg bg-white/5 px-3 py-3">
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-sm text-white/90">Version</span>
                    <span className="font-mono text-sm text-white/70">
                      {version ? `v${version}` : "—"}
                    </span>
                  </div>
                  <div className="mt-3 border-t border-white/10 pt-3">
                    <button
                      type="button"
                      onClick={() => void handleOpenGitHub()}
                      className="w-full text-left text-sm text-sky-200 transition hover:text-sky-100"
                      title={GITHUB_URL}
                    >
                      GitHub project page →
                    </button>
                    <p className="mt-1 break-all text-xs text-white/40">{GITHUB_URL}</p>
                  </div>
                </div>
                {linkError && (
                  <p className="mt-2 text-xs text-red-300">{linkError}</p>
                )}
              </section>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}