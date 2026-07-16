import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { copyPngToClipboard, getLastSnipCapture, saveEditedSnip } from "../api";
import { base64FromDataUrl } from "../lib/imageDataUrl";
import type { CaptureResult } from "../types";

type Tool = "pen" | "highlighter" | "blur" | "crop";

const COLORS = [
  { id: "red", hex: "#ef4444" },
  { id: "orange", hex: "#f97316" },
  { id: "yellow", hex: "#eab308" },
  { id: "green", hex: "#22c55e" },
  { id: "blue", hex: "#3b82f6" },
  { id: "purple", hex: "#a855f7" },
  { id: "black", hex: "#111827" },
  { id: "white", hex: "#f8fafc" },
] as const;

type Point = { x: number; y: number };
type Rect = { x: number; y: number; w: number; h: number };

function hexToRgba(hex: string, alpha: number): string {
  const h = hex.replace("#", "");
  const n = parseInt(h, 16);
  const r = (n >> 16) & 255;
  const g = (n >> 8) & 255;
  const b = n & 255;
  return `rgba(${r},${g},${b},${alpha})`;
}

function normalizeRect(a: Point, b: Point): Rect {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  const w = Math.abs(b.x - a.x);
  const h = Math.abs(b.y - a.y);
  return { x, y, w, h };
}

/** Fast separable box blur on a region of ImageData. */
function boxBlurRegion(
  data: Uint8ClampedArray,
  width: number,
  height: number,
  rx: number,
  ry: number,
  rw: number,
  rh: number,
  radius: number,
) {
  if (radius < 1 || rw < 1 || rh < 1) return;
  const tmp = new Uint8ClampedArray(rw * rh * 4);
  const src = data;

  const clampX = (x: number) => Math.max(0, Math.min(width - 1, x));
  const clampY = (y: number) => Math.max(0, Math.min(height - 1, y));

  // Horizontal
  for (let y = 0; y < rh; y++) {
    for (let x = 0; x < rw; x++) {
      let r = 0,
        g = 0,
        b = 0,
        a = 0,
        count = 0;
      for (let k = -radius; k <= radius; k++) {
        const sx = clampX(rx + x + k);
        const sy = clampY(ry + y);
        const i = (sy * width + sx) * 4;
        r += src[i];
        g += src[i + 1];
        b += src[i + 2];
        a += src[i + 3];
        count++;
      }
      const ti = (y * rw + x) * 4;
      tmp[ti] = r / count;
      tmp[ti + 1] = g / count;
      tmp[ti + 2] = b / count;
      tmp[ti + 3] = a / count;
    }
  }

  // Vertical back into src
  for (let y = 0; y < rh; y++) {
    for (let x = 0; x < rw; x++) {
      let r = 0,
        g = 0,
        b = 0,
        a = 0,
        count = 0;
      for (let k = -radius; k <= radius; k++) {
        const sy = Math.max(0, Math.min(rh - 1, y + k));
        const ti = (sy * rw + x) * 4;
        r += tmp[ti];
        g += tmp[ti + 1];
        b += tmp[ti + 2];
        a += tmp[ti + 3];
        count++;
      }
      const di = ((ry + y) * width + (rx + x)) * 4;
      src[di] = r / count;
      src[di + 1] = g / count;
      src[di + 2] = b / count;
      src[di + 3] = a / count;
    }
  }
}

export function SnipEditor() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLCanvasElement>(null);
  const [tool, setTool] = useState<Tool>("pen");
  const [color, setColor] = useState<string>(COLORS[0].hex);
  const [capture, setCapture] = useState<CaptureResult | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [canUndo, setCanUndo] = useState(false);

  const drawing = useRef(false);
  const lastPoint = useRef<Point | null>(null);
  const dragStart = useRef<Point | null>(null);
  const undoStack = useRef<ImageData[]>([]);
  const maxUndo = 30;

  const getCtx = () => canvasRef.current?.getContext("2d", { willReadFrequently: true }) ?? null;
  const getOverlayCtx = () => overlayRef.current?.getContext("2d") ?? null;

  const pushUndo = useCallback(() => {
    const canvas = canvasRef.current;
    const ctx = getCtx();
    if (!canvas || !ctx) return;
    try {
      const snap = ctx.getImageData(0, 0, canvas.width, canvas.height);
      undoStack.current.push(snap);
      if (undoStack.current.length > maxUndo) undoStack.current.shift();
      setCanUndo(undoStack.current.length > 0);
    } catch {
      // tainted canvas unlikely for data URLs
    }
  }, []);

  const loadCapture = useCallback((next: CaptureResult) => {
    if (!next?.pngBase64) return;
    setCapture(next);
    undoStack.current = [];
    setCanUndo(false);
    setStatus(null);

    const applyToCanvas = () => {
      const canvas = canvasRef.current;
      const overlay = overlayRef.current;
      if (!canvas) return;
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const image = new Image();
      image.onload = () => {
        canvas.width = image.width;
        canvas.height = image.height;
        if (overlay) {
          overlay.width = image.width;
          overlay.height = image.height;
          overlay.getContext("2d")?.clearRect(0, 0, overlay.width, overlay.height);
        }
        ctx.drawImage(image, 0, 0);
      };
      image.src = `data:image/png;base64,${next.pngBase64}`;
    };

    // Canvas may not be mounted on first paint of a just-shown window.
    requestAnimationFrame(() => applyToCanvas());
  }, []);

  const pullLastCapture = useCallback(async () => {
    try {
      const last = await getLastSnipCapture();
      if (last?.pngBase64) {
        loadCapture(last);
      }
    } catch (err) {
      console.error("get_last_snip_capture failed:", err);
    }
  }, [loadCapture]);

  useEffect(() => {
    // Pull stored snip immediately (covers missed push events).
    void pullLastCapture();

    const unlisten = listen<CaptureResult>("editor-image", (event) => {
      loadCapture(event.payload);
      void getCurrentWindow().show();
      void getCurrentWindow().setFocus();
    });

    let unlistenFocus: (() => void) | undefined;
    void (async () => {
      const win = getCurrentWindow();
      unlistenFocus = await win.onFocusChanged(({ payload: focused }) => {
        if (focused) {
          // If we opened empty, re-pull after the webview is focused.
          void getLastSnipCapture().then((last) => {
            if (last?.pngBase64) loadCapture(last);
          });
        }
      });
    })();

    return () => {
      void unlisten.then((fn) => fn());
      unlistenFocus?.();
    };
  }, [loadCapture, pullLastCapture]);

  const getPoint = (event: React.PointerEvent<HTMLCanvasElement>): Point | null => {
    const canvas = canvasRef.current;
    if (!canvas) return null;
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (event.clientX - rect.left) * scaleX,
      y: (event.clientY - rect.top) * scaleY,
    };
  };

  const clearOverlay = () => {
    const overlay = overlayRef.current;
    const octx = getOverlayCtx();
    if (!overlay || !octx) return;
    octx.clearRect(0, 0, overlay.width, overlay.height);
  };

  const drawOverlayRect = (rect: Rect, fill: string, stroke: string) => {
    const octx = getOverlayCtx();
    const overlay = overlayRef.current;
    if (!octx || !overlay) return;
    octx.clearRect(0, 0, overlay.width, overlay.height);
    octx.fillStyle = fill;
    octx.strokeStyle = stroke;
    octx.lineWidth = 2;
    octx.fillRect(rect.x, rect.y, rect.w, rect.h);
    octx.strokeRect(rect.x, rect.y, rect.w, rect.h);
  };

  const onPointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (!capture) return;
    const point = getPoint(event);
    if (!point) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    drawing.current = true;
    lastPoint.current = point;
    dragStart.current = point;

    if (tool === "pen" || tool === "highlighter") {
      pushUndo();
      const ctx = getCtx();
      if (!ctx) return;
      ctx.lineCap = "round";
      ctx.lineJoin = "round";
      if (tool === "pen") {
        ctx.globalCompositeOperation = "source-over";
        ctx.strokeStyle = color;
        ctx.lineWidth = 3;
      } else {
        ctx.globalCompositeOperation = "multiply";
        ctx.strokeStyle = hexToRgba(color, 0.45);
        ctx.lineWidth = 18;
      }
      ctx.beginPath();
      ctx.moveTo(point.x, point.y);
      ctx.lineTo(point.x + 0.01, point.y);
      ctx.stroke();
    }
  };

  const onPointerMove = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (!drawing.current || !lastPoint.current) return;
    const point = getPoint(event);
    if (!point) return;

    if (tool === "pen" || tool === "highlighter") {
      const ctx = getCtx();
      if (!ctx) return;
      ctx.beginPath();
      ctx.moveTo(lastPoint.current.x, lastPoint.current.y);
      ctx.lineTo(point.x, point.y);
      ctx.stroke();
      lastPoint.current = point;
      return;
    }

    if ((tool === "blur" || tool === "crop") && dragStart.current) {
      const rect = normalizeRect(dragStart.current, point);
      if (tool === "blur") {
        drawOverlayRect(rect, "rgba(148,163,184,0.25)", "#94a3b8");
      } else {
        drawOverlayRect(rect, "rgba(56,189,248,0.15)", "#38bdf8");
      }
    }
  };

  const applyBlur = (rect: Rect) => {
    const canvas = canvasRef.current;
    const ctx = getCtx();
    if (!canvas || !ctx || rect.w < 2 || rect.h < 2) return;
    pushUndo();
    const x = Math.max(0, Math.floor(rect.x));
    const y = Math.max(0, Math.floor(rect.y));
    const w = Math.min(canvas.width - x, Math.ceil(rect.w));
    const h = Math.min(canvas.height - y, Math.ceil(rect.h));
    if (w < 2 || h < 2) return;
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    // A few passes for a softer blur.
    for (let i = 0; i < 3; i++) {
      boxBlurRegion(imageData.data, canvas.width, canvas.height, x, y, w, h, 4);
    }
    ctx.putImageData(imageData, 0, 0);
  };

  const applyCrop = (rect: Rect) => {
    const canvas = canvasRef.current;
    const ctx = getCtx();
    const overlay = overlayRef.current;
    if (!canvas || !ctx || rect.w < 4 || rect.h < 4) return;
    pushUndo();
    const x = Math.max(0, Math.floor(rect.x));
    const y = Math.max(0, Math.floor(rect.y));
    const w = Math.min(canvas.width - x, Math.ceil(rect.w));
    const h = Math.min(canvas.height - y, Math.ceil(rect.h));
    const cropped = ctx.getImageData(x, y, w, h);
    canvas.width = w;
    canvas.height = h;
    if (overlay) {
      overlay.width = w;
      overlay.height = h;
    }
    ctx.putImageData(cropped, 0, 0);
  };

  const onPointerUp = (event: React.PointerEvent<HTMLCanvasElement>) => {
    if (!drawing.current) return;
    drawing.current = false;
    try {
      event.currentTarget.releasePointerCapture(event.pointerId);
    } catch {
      /* ignore */
    }

    const point = getPoint(event) ?? lastPoint.current;
    if ((tool === "blur" || tool === "crop") && dragStart.current && point) {
      const rect = normalizeRect(dragStart.current, point);
      clearOverlay();
      if (tool === "blur") applyBlur(rect);
      else applyCrop(rect);
    }

    const ctx = getCtx();
    if (ctx) ctx.globalCompositeOperation = "source-over";
    lastPoint.current = null;
    dragStart.current = null;
  };

  const handleUndo = () => {
    const canvas = canvasRef.current;
    const ctx = getCtx();
    const snap = undoStack.current.pop();
    if (!canvas || !ctx || !snap) return;
    if (canvas.width !== snap.width || canvas.height !== snap.height) {
      canvas.width = snap.width;
      canvas.height = snap.height;
      const overlay = overlayRef.current;
      if (overlay) {
        overlay.width = snap.width;
        overlay.height = snap.height;
      }
    }
    ctx.putImageData(snap, 0, 0);
    setCanUndo(undoStack.current.length > 0);
  };

  const exportPng = (): string => {
    const canvas = canvasRef.current;
    if (!canvas) return "";
    const dataUrl = canvas.toDataURL("image/png");
    return base64FromDataUrl(dataUrl);
  };

  const handleCopy = async () => {
    const png = exportPng();
    if (!png) return;
    try {
      await copyPngToClipboard(png);
      setStatus("Copied to clipboard");
    } catch (err) {
      console.error(err);
      setStatus("Copy failed");
    }
  };

  const handleSave = async () => {
    const png = exportPng();
    if (!png) return;
    try {
      const result = await saveEditedSnip(png, capture?.savedPath);
      setCapture(result);
      setStatus(result.savedPath ? `Saved · ${result.savedPath}` : "Saved");
    } catch (err) {
      console.error(err);
      setStatus("Save failed");
    }
  };

  const handleClose = async () => {
    await getCurrentWindow().hide();
  };

  const cursorClass =
    tool === "crop" || tool === "blur"
      ? "cursor-crosshair"
      : tool === "pen" || tool === "highlighter"
        ? "cursor-crosshair"
        : "cursor-default";

  return (
    <div className="flex h-screen flex-col bg-neutral-950 text-white">
      <header className="flex flex-wrap items-center gap-2 border-b border-white/10 px-3 py-2">
        {(
          [
            ["pen", "Pen"],
            ["highlighter", "Highlight"],
            ["blur", "Blur"],
            ["crop", "Crop"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            type="button"
            onClick={() => {
              setTool(id);
              clearOverlay();
            }}
            className={`rounded-md px-3 py-1.5 text-sm ${
              tool === id ? "bg-sky-500/30 text-sky-100" : "hover:bg-white/10"
            }`}
          >
            {label}
          </button>
        ))}

        <div className="mx-1 h-6 w-px bg-white/10" />

        <div className="flex items-center gap-1.5">
          {COLORS.map((c) => (
            <button
              key={c.id}
              type="button"
              title={c.id}
              onClick={() => setColor(c.hex)}
              className={`h-6 w-6 rounded-full border-2 ${
                color === c.hex ? "border-white scale-110" : "border-white/20"
              }`}
              style={{ backgroundColor: c.hex }}
            />
          ))}
        </div>

        <button
          type="button"
          onClick={handleUndo}
          disabled={!canUndo}
          className="rounded-md px-3 py-1.5 text-sm hover:bg-white/10 disabled:opacity-30"
        >
          Undo
        </button>

        <div className="ml-auto flex items-center gap-2">
          {status && <span className="max-w-[240px] truncate text-xs text-white/50">{status}</span>}
          <button
            type="button"
            onClick={() => void handleCopy()}
            className="rounded-md px-3 py-1.5 text-sm hover:bg-white/10"
          >
            Copy
          </button>
          <button
            type="button"
            onClick={() => void handleSave()}
            className="rounded-md bg-sky-600 px-3 py-1.5 text-sm hover:bg-sky-500"
          >
            Save
          </button>
          <button
            type="button"
            onClick={() => void handleClose()}
            className="rounded-md px-3 py-1.5 text-sm hover:bg-white/10"
          >
            Close
          </button>
        </div>
      </header>

      <div className="relative flex flex-1 items-center justify-center overflow-auto bg-neutral-900/80 p-4">
        {!capture && (
          <p className="text-sm text-white/50">No snip loaded. Capture a snip and click the toast to edit.</p>
        )}
        <div className="relative inline-block max-h-full max-w-full">
          <canvas
            ref={canvasRef}
            className={`block max-h-[calc(100vh-5rem)] max-w-full rounded-lg border border-white/10 bg-black ${cursorClass}`}
            onPointerDown={onPointerDown}
            onPointerMove={onPointerMove}
            onPointerUp={onPointerUp}
            onPointerCancel={onPointerUp}
          />
          <canvas
            ref={overlayRef}
            className="pointer-events-none absolute left-0 top-0 h-full w-full rounded-lg"
          />
        </div>
      </div>

      <footer className="border-t border-white/10 px-3 py-1.5 text-xs text-white/40">
        {tool === "crop" && "Drag a rectangle to crop. Release to apply."}
        {tool === "blur" && "Drag a rectangle to blur that area."}
        {tool === "pen" && "Draw freehand. Pick a color above."}
        {tool === "highlighter" && "Highlight freehand with a translucent stroke."}
        {" · Save updates clipboard and the Screenshots file."}
      </footer>
    </div>
  );
}
