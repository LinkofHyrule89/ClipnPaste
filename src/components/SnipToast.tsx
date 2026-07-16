import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openSnipEditor } from "../api";
import type { CaptureResult } from "../types";

export function SnipToast() {
  const [capture, setCapture] = useState<CaptureResult | null>(null);

  useEffect(() => {
    const unlisten = listen<CaptureResult>("snip-captured", (event) => {
      setCapture(event.payload);
      void getCurrentWindow().show();
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (!capture) return;
    const timer = window.setTimeout(() => {
      void getCurrentWindow().hide();
    }, 8000);
    return () => window.clearTimeout(timer);
  }, [capture]);

  const openEditor = async () => {
    if (!capture) return;
    try {
      // Open editor first (stores last_capture + retries emit), then hide toast.
      await openSnipEditor(capture);
    } catch (err) {
      console.error("Failed to open snip editor:", err);
    }
    await getCurrentWindow().hide();
  };

  if (!capture) {
    return <div className="h-screen w-screen bg-transparent" />;
  }

  return (
    <button
      type="button"
      onClick={() => void openEditor()}
      className="glass-panel flex h-full w-full items-center gap-3 px-3 text-left text-white"
    >
      <img
        src={`data:image/png;base64,${capture.pngBase64}`}
        alt="Captured snip"
        className="h-16 w-20 rounded-md border border-white/10 object-cover"
      />
      <div>
        <p className="text-sm font-medium">Snip copied and saved</p>
        <p className="text-xs text-white/50">Saved to Pictures/Screenshots · Click to edit</p>
      </div>
    </button>
  );
}
