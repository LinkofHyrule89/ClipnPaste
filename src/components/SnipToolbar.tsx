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