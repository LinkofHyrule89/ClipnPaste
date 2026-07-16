# Security

ClipnPaste is a **local desktop** app for Linux (X11). It does not require network access at runtime and does not phone home.

## Threat model

| In scope | Out of scope |
|----------|----------------|
| Malicious or buggy frontend invoke trying to write outside Screenshots | Full multi-user / multi-seat isolation on a shared machine |
| Accidental path traversal via save filename/path | Defending against other processes running **as the same user** |
| Overly large clipboard/image payloads exhausting memory | Kernel / X11 / compositor compromise |
| Loose Unix socket permissions for local IPC | Network attackers (app is offline) |

Same-user malware can already read the clipboard and drive the session; ClipnPaste does not claim to stop that.

## Data on disk

| Path | Mode (intent) | Contents |
|------|----------------|----------|
| `~/.local/share/clipnpaste/` | `0700` | App data directory |
| `…/history.db` | `0600` | Clipboard history (text + images as data URLs) |
| `…/settings.json` | user umask | UI preferences |
| `…/ipc.sock` | `0600` | Local control socket for hotkeys / `clipnpaste-cli` |
| `…/focus_target` | under data dir | Last paste target window id |
| `$XDG_PICTURES_DIR/Screenshots/` | user dir | Snip PNG files |

Treat history and screenshots as **sensitive** (passwords, secrets, personal images).

## IPC

`clipnpaste-cli` and desktop shortcuts talk to a **Unix domain socket** under the data directory. Commands are allowlisted: `clipboard`, `emoji`, `snip`, `settings`. There is no authentication beyond filesystem permissions (owner-only socket).

## Snip save paths

Edited snips and `save_png` only write under the Screenshots directory. Client-supplied paths are validated (basename sanitization; no `..` / absolute escape). Decoded PNG size is capped (`MAX_IMAGE_BYTES`, 10 MiB).

## Webview / CSP

Tauri CSP allows `script-src 'self' 'unsafe-inline'` (typical for this stack). Windows are local UI only; no remote origins are loaded for core features.

## Reporting

If you find a vulnerability, open a private security advisory or contact the maintainer via the GitHub repository. Please avoid filing public issues that include exploit details until a fix is available.
