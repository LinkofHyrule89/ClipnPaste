import { ClipboardPanel } from "./components/ClipboardPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { SnipEditor } from "./components/SnipEditor";
import { SnipOverlay } from "./components/SnipOverlay";
import { SnipToast } from "./components/SnipToast";
import { SnipToolbar } from "./components/SnipToolbar";

function getWindowKind() {
  const params = new URLSearchParams(window.location.search);
  return params.get("window") ?? "main";
}

export default function App() {
  const kind = getWindowKind();

  switch (kind) {
    case "clipboard":
      return <ClipboardPanel />;
    case "settings":
      return <SettingsPanel />;
    case "snip-toolbar":
      return <SnipToolbar />;
    case "snip-overlay":
      return <SnipOverlay />;
    case "snip-toast":
      return <SnipToast />;
    case "snip-editor":
      return <SnipEditor />;
    default:
      return (
        <div className="flex h-screen items-center justify-center bg-neutral-950 text-white">
          <p className="text-sm text-white/60">ClipnPaste is running in the tray.</p>
        </div>
      );
  }
}