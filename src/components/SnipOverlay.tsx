import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listCaptureWindows, snipRegion, snipWindow } from "../api";
import type { WindowInfo } from "../types";

type DragState = {
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
};

const waitFrames = (n: number) =>
  new Promise<void>((resolve) => {
    const step = (left: number) => {
      if (left <= 0) {
        resolve();
        return;
      }
      requestAnimationFrame(() => step(left - 1));
    };
    step(n);
  });

export function SnipOverlay() {
  const [mode, setMode] = useState<"rect" | "window">("rect");
  const [drag, setDrag] = useState<DragState | null>(null);
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  /** When true, paint nothing so a late compositor frame has no chrome. */
  const [captureReady, setCaptureReady] = useState(false);
  const dragRef = useRef<DragState | null>(null);
  const capturingRef = useRef(false);

  useEffect(() => {
    const unlisten = listen<string>("snip-mode", (event) => {
      if (event.payload === "window") {
        setMode("window");
        setCaptureReady(false);
        setDrag(null);
        dragRef.current = null;
        void listCaptureWindows().then(setWindows);
      } else if (event.payload === "rect") {
        setMode("rect");
        setCaptureReady(false);
        setWindows([]);
        setDrag(null);
        dragRef.current = null;
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const hide = async () => {
    await getCurrentWindow().hide();
  };

  const updateDrag = (next: DragState | null) => {
    dragRef.current = next;
    setDrag(next);
  };

  /** Clear selection chrome, go fully transparent, hide window. Rust also hides + settles. */
  const prepareAndHide = async () => {
    updateDrag(null);
    setCaptureReady(true);
    await waitFrames(2);
    await hide();
  };

  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    if (mode !== "rect" || capturingRef.current) return;
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    updateDrag({
      startX: event.clientX,
      startY: event.clientY,
      currentX: event.clientX,
      currentY: event.clientY,
    });
  };

  const onPointerMove = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!dragRef.current || mode !== "rect") return;
    updateDrag({
      ...dragRef.current,
      currentX: event.clientX,
      currentY: event.clientY,
    });
  };

  const onPointerUp = async (event: React.PointerEvent<HTMLDivElement>) => {
    if (mode !== "rect" || capturingRef.current) return;
    const active = dragRef.current;
    if (!active) return;

    try {
      event.currentTarget.releasePointerCapture(event.pointerId);
    } catch {
      // Capture may already be released.
    }

    const left = Math.min(active.startX, active.currentX);
    const top = Math.min(active.startY, active.currentY);
    const width = Math.abs(active.currentX - active.startX);
    const height = Math.abs(active.currentY - active.startY);
    updateDrag(null);

    if (width < 4 || height < 4) {
      await prepareAndHide();
      setCaptureReady(false);
      return;
    }

    capturingRef.current = true;
    try {
      // Map CSS viewport coords → physical screen pixels.
      const win = getCurrentWindow();
      const pos = await win.outerPosition();
      const phys = await win.innerSize();
      const cssW = Math.max(1, window.innerWidth);
      const cssH = Math.max(1, window.innerHeight);
      const scaleX = phys.width / cssW;
      const scaleY = phys.height / cssH;
      const screenX = Math.round(pos.x + left * scaleX);
      const screenY = Math.round(pos.y + top * scaleY);
      const physW = Math.max(1, Math.round(width * scaleX));
      const physH = Math.max(1, Math.round(height * scaleY));

      // Clear UI first; Rust hides overlay + XSync + settle before GetImage.
      await prepareAndHide();
      await snipRegion(screenX, screenY, physW, physH);
    } catch (err) {
      console.error("Region snip failed:", err);
      await hide();
    } finally {
      capturingRef.current = false;
      setCaptureReady(false);
    }
  };

  const captureWindow = async (windowId: number) => {
    if (capturingRef.current) return;
    capturingRef.current = true;
    try {
      await prepareAndHide();
      await snipWindow(windowId);
    } catch (err) {
      console.error("Window snip failed:", err);
      await hide();
    } finally {
      capturingRef.current = false;
      setCaptureReady(false);
    }
  };

  useEffect(() => {
    const onKey = async (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        updateDrag(null);
        setCaptureReady(false);
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
      className={
        captureReady
          ? "relative h-screen w-screen cursor-crosshair bg-transparent select-none touch-none opacity-0"
          : "relative h-screen w-screen cursor-crosshair bg-black/35 select-none touch-none"
      }
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={(e) => void onPointerUp(e)}
      onPointerCancel={() => updateDrag(null)}
    >
      {mode === "window" && !captureReady && (
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

      {selectionStyle && !captureReady && (
        <div
          className="pointer-events-none absolute border-2 border-sky-400 bg-sky-400/10"
          style={selectionStyle}
        />
      )}
    </div>
  );
}
