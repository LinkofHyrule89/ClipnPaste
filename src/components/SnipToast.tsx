import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
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
    const editor = await WebviewWindow.getByLabel("snip-editor");
    if (editor) {
      await editor.emit("editor-image", capture);
      await editor.show();
      await editor.setFocus();
    }
    await getCurrentWindow().hide();
  };

  if (!capture) {
    return <div className="h-screen w-screen bg-transparent" />;
  }

  return (
    <button
      onClick={() => void openEditor()}
      className="glass-panel flex h-full w-full items-center gap-3 px-3 text-left text-white"
    >
      <img
        src={`data:image/png;base64,${capture.pngBase64}`}
        alt="Captured snip"
        className="h-16 w-20 rounded-md border border-white/10 object-cover"
      />
      <div>
        <p className="text-sm font-medium">Snip copied to clipboard</p>
        <p className="text-xs text-white/50">Click to edit</p>
      </div>
    </button>
  );
}