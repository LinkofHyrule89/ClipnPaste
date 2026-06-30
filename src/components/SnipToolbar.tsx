import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { snipFullscreen } from "../api";

type SnipMode = "rect" | "window" | "screen" | null;

export function SnipToolbar() {
  const hide = async () => {
    await getCurrentWindow().hide();
  };

  const startMode = async (mode: SnipMode) => {
    if (mode === "screen") {
      await hide();
      await snipFullscreen();
      return;
    }

    if (mode === "rect") {
      await hide();
      const overlay = await WebviewWindow.getByLabel("snip-overlay");
      if (overlay) {
        await overlay.show();
        await overlay.setFocus();
      }
      return;
    }

    if (mode === "window") {
      await hide();
      const overlay = await WebviewWindow.getByLabel("snip-overlay");
      if (overlay) {
        await overlay.emit("snip-mode", "window");
        await overlay.show();
        await overlay.setFocus();
      }
    }
  };

  return (
    <div className="flex h-screen w-screen items-center justify-center bg-transparent p-2">
      <div className="glass-panel flex items-center gap-1 px-2 py-2">
        <button
          className="toolbar-btn"
          title="Rectangle snip"
          onClick={() => void startMode("rect")}
        >
          ▢
        </button>
        <button
          className="toolbar-btn"
          title="Window snip"
          onClick={() => void startMode("window")}
        >
          ⧉
        </button>
        <button
          className="toolbar-btn"
          title="Fullscreen snip"
          onClick={() => void startMode("screen")}
        >
          ⛶
        </button>
        <div className="mx-1 h-6 w-px bg-white/10" />
        <button className="toolbar-btn" title="Close" onClick={() => void hide()}>
          ✕
        </button>
      </div>
    </div>
  );
}