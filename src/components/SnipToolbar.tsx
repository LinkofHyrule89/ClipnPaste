import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { snipFullscreen } from "../api";

type SnipMode = "rect" | "window" | "screen" | null;

export function SnipToolbar() {
  const hide = async () => {
    await getCurrentWindow().hide();
  };

  const startDrag = (event: React.MouseEvent) => {
    if (event.button === 0) {
      void getCurrentWindow().startDragging();
    }
  };

  useEffect(() => {
    void getCurrentWindow().setAlwaysOnTop(true);
  }, []);

  const startMode = async (mode: SnipMode) => {
    if (mode === "screen") {
      await hide();
      await snipFullscreen();
      return;
    }

    if (mode === "rect" || mode === "window") {
      await hide();
      const overlay = await WebviewWindow.getByLabel("snip-overlay");
      if (overlay) {
        // Overlay webview stays mounted while hidden, so mode must be set
        // every time (otherwise a prior window snip leaves list UI active).
        await overlay.emit("snip-mode", mode);
        await overlay.show();
        await overlay.setFocus();
      }
    }
  };

  return (
    <div className="inline-flex items-center gap-0.5 p-0.5">
      <button
        type="button"
        className="snip-drag-handle"
        title="Drag to move"
        aria-label="Drag to move"
        onMouseDown={startDrag}
      >
        ⠿
      </button>
      <button
        type="button"
        className="snip-toolbar-btn"
        title="Rectangle snip"
        onClick={() => void startMode("rect")}
      >
        ▢
      </button>
      <button
        type="button"
        className="snip-toolbar-btn"
        title="Window snip"
        onClick={() => void startMode("window")}
      >
        ⧉
      </button>
      <button
        type="button"
        className="snip-toolbar-btn"
        title="Fullscreen snip"
        onClick={() => void startMode("screen")}
      >
        ⛶
      </button>
      <button
        type="button"
        className="snip-toolbar-btn"
        title="Close"
        onClick={() => void hide()}
      >
        ✕
      </button>
    </div>
  );
}