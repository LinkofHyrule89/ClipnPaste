import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listCaptureWindows, snipRegion, snipWindow } from "../api";
import type { WindowInfo } from "../types";

type DragState = {
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
};

export function SnipOverlay() {
  const [mode, setMode] = useState<"rect" | "window">("rect");
  const [drag, setDrag] = useState<DragState | null>(null);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = listen<string>("snip-mode", (event) => {
      if (event.payload === "window") {
        setMode("window");
        void listCaptureWindows().then(setWindows);
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const hide = async () => {
    await getCurrentWindow().hide();
  };

  const showToast = async () => {
    const toast = await WebviewWindow.getByLabel("snip-toast");
    await toast?.show();
  };

  const onMouseDown = (event: React.MouseEvent) => {
    if (mode !== "rect") return;
    setDrag({
      startX: event.clientX,
      startY: event.clientY,
      currentX: event.clientX,
      currentY: event.clientY,
    });
  };

  const onMouseMove = (event: React.MouseEvent) => {
    if (!drag) return;
    setDrag({ ...drag, currentX: event.clientX, currentY: event.clientY });
  };

  const onMouseUp = async () => {
    if (!drag || mode !== "rect") return;
    const left = Math.min(drag.startX, drag.currentX);
    const top = Math.min(drag.startY, drag.currentY);
    const width = Math.abs(drag.currentX - drag.startX);
    const height = Math.abs(drag.currentY - drag.startY);
    setDrag(null);

    if (width < 4 || height < 4) {
      await hide();
      return;
    }

    const rect = overlayRef.current?.getBoundingClientRect();
    const offsetX = rect?.left ?? 0;
    const offsetY = rect?.top ?? 0;
    const screenX = Math.round(left + offsetX + window.screenX);
    const screenY = Math.round(top + offsetY + window.screenY);

    await snipRegion(screenX, screenY, Math.round(width), Math.round(height));
    await hide();
    await showToast();
  };

  const captureWindow = async (windowId: number) => {
    await snipWindow(windowId);
    await hide();
    await showToast();
  };

  useEffect(() => {
    const onKey = async (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        await hide();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const selectionStyle = drag
    ? {
        left: Math.min(drag.startX, drag.currentX),
        top: Math.min(drag.startY, drag.currentY),
        width: Math.abs(drag.currentX - drag.startX),
        height: Math.abs(drag.currentY - drag.startY),
      }
    : null;

  return (
    <div
      ref={overlayRef}
      className="relative h-screen w-screen cursor-crosshair bg-black/35"
      onMouseDown={onMouseDown}
      onMouseMove={onMouseMove}
      onMouseUp={() => void onMouseUp()}
    >
      {mode === "window" && (
        <div className="absolute left-1/2 top-16 z-20 max-h-[70vh] w-[420px] -translate-x-1/2 overflow-y-auto rounded-xl border border-white/10 bg-neutral-900/95 p-3 text-white shadow-2xl">
          <p className="mb-3 text-sm font-medium">Select a window</p>
          {windows.map((item) => (
            <button
              key={item.id}
              onClick={() => void captureWindow(item.id)}
              className="mb-2 block w-full rounded-lg px-3 py-2 text-left text-sm hover:bg-white/10"
            >
              <span className="block font-medium">{item.title || "Untitled"}</span>
              <span className="text-xs text-white/50">{item.appName}</span>
            </button>
          ))}
        </div>
      )}

      {selectionStyle && (
        <div
          className="pointer-events-none absolute border-2 border-sky-400 bg-sky-400/10"
          style={selectionStyle}
        />
      )}
    </div>
  );
}