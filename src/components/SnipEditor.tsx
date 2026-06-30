import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { copyPngToClipboard, savePng } from "../api";
import type { CaptureResult } from "../types";

type Tool = "pen" | "highlighter" | "rect" | "arrow";

export function SnipEditor() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [tool, setTool] = useState<Tool>("pen");
  const [drawing, setDrawing] = useState(false);
  const [capture, setCapture] = useState<CaptureResult | null>(null);
  const lastPoint = useRef<{ x: number; y: number } | null>(null);

  useEffect(() => {
    const unlisten = listen<CaptureResult>("editor-image", (event) => {
      setCapture(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!capture || !canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const image = new Image();
    image.onload = () => {
      canvas.width = image.width;
      canvas.height = image.height;
      ctx.drawImage(image, 0, 0);
    };
    image.src = `data:image/png;base64,${capture.pngBase64}`;
  }, [capture]);

  const getPoint = (event: React.MouseEvent<HTMLCanvasElement>) => {
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

  const startDraw = (event: React.MouseEvent<HTMLCanvasElement>) => {
    const point = getPoint(event);
    if (!point) return;
    setDrawing(true);
    lastPoint.current = point;
  };

  const draw = (event: React.MouseEvent<HTMLCanvasElement>) => {
    if (!drawing || !canvasRef.current || !lastPoint.current) return;
    const ctx = canvasRef.current.getContext("2d");
    const point = getPoint(event);
    if (!ctx || !point) return;

    ctx.lineCap = "round";
    ctx.lineJoin = "round";

    if (tool === "pen") {
      ctx.strokeStyle = "#f43f5e";
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(lastPoint.current.x, lastPoint.current.y);
      ctx.lineTo(point.x, point.y);
      ctx.stroke();
    } else if (tool === "highlighter") {
      ctx.strokeStyle = "rgba(250, 204, 21, 0.55)";
      ctx.lineWidth = 14;
      ctx.beginPath();
      ctx.moveTo(lastPoint.current.x, lastPoint.current.y);
      ctx.lineTo(point.x, point.y);
      ctx.stroke();
    } else if (tool === "rect") {
      const start = lastPoint.current;
      ctx.clearRect(0, 0, canvasRef.current.width, canvasRef.current.height);
      const image = new Image();
      image.src = `data:image/png;base64,${capture?.pngBase64 ?? ""}`;
      image.onload = () => {
        ctx.drawImage(image, 0, 0);
        ctx.strokeStyle = "#38bdf8";
        ctx.lineWidth = 3;
        ctx.strokeRect(
          start.x,
          start.y,
          point.x - start.x,
          point.y - start.y,
        );
      };
    } else if (tool === "arrow") {
      const start = lastPoint.current;
      ctx.clearRect(0, 0, canvasRef.current.width, canvasRef.current.height);
      const image = new Image();
      image.src = `data:image/png;base64,${capture?.pngBase64 ?? ""}`;
      image.onload = () => {
        ctx.drawImage(image, 0, 0);
        ctx.strokeStyle = "#38bdf8";
        ctx.fillStyle = "#38bdf8";
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.moveTo(start.x, start.y);
        ctx.lineTo(point.x, point.y);
        ctx.stroke();
        const angle = Math.atan2(point.y - start.y, point.x - start.x);
        const head = 12;
        ctx.beginPath();
        ctx.moveTo(point.x, point.y);
        ctx.lineTo(
          point.x - head * Math.cos(angle - Math.PI / 6),
          point.y - head * Math.sin(angle - Math.PI / 6),
        );
        ctx.lineTo(
          point.x - head * Math.cos(angle + Math.PI / 6),
          point.y - head * Math.sin(angle + Math.PI / 6),
        );
        ctx.closePath();
        ctx.fill();
      };
    }

    lastPoint.current = point;
  };

  const endDraw = () => {
    setDrawing(false);
    lastPoint.current = null;
  };

  const exportPng = async () => {
    const canvas = canvasRef.current;
    if (!canvas) return "";
    const dataUrl = canvas.toDataURL("image/png");
    return dataUrl.split(",")[1] ?? "";
  };

  const handleCopy = async () => {
    const png = await exportPng();
    if (png) await copyPngToClipboard(png);
  };

  const handleSave = async () => {
    const png = await exportPng();
    if (png) await savePng(png);
  };

  return (
    <div className="flex h-screen flex-col bg-neutral-950 text-white">
      <header className="flex items-center gap-2 border-b border-white/10 px-4 py-3">
        {(["pen", "highlighter", "rect", "arrow"] as Tool[]).map((value) => (
          <button
            key={value}
            onClick={() => setTool(value)}
            className={`rounded-md px-3 py-1.5 text-sm capitalize ${
              tool === value ? "bg-sky-500/30" : "hover:bg-white/10"
            }`}
          >
            {value}
          </button>
        ))}
        <div className="ml-auto flex gap-2">
          <button
            onClick={() => void handleCopy()}
            className="rounded-md bg-sky-600 px-3 py-1.5 text-sm hover:bg-sky-500"
          >
            Copy
          </button>
          <button
            onClick={() => void handleSave()}
            className="rounded-md px-3 py-1.5 text-sm hover:bg-white/10"
          >
            Save
          </button>
        </div>
      </header>
      <div className="flex flex-1 items-center justify-center overflow-auto p-4">
        <canvas
          ref={canvasRef}
          className="max-h-full max-w-full rounded-lg border border-white/10 bg-black"
          onMouseDown={startDraw}
          onMouseMove={draw}
          onMouseUp={endDraw}
          onMouseLeave={endDraw}
        />
      </div>
    </div>
  );
}