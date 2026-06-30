# ClipnPaste

Windows 11-style clipboard history and snipping tool for Linux Mint and other Debian-based desktops.

## Features

- `Super+V` opens clipboard history with text and image previews
- Pin items and clear all unpinned entries
- `Super+Shift+S` opens the snipping toolbar
- Capture fullscreen, a single window, or a selected region
- Snips copy to clipboard immediately and show a toast preview
- Click the toast to open the annotation editor

## Requirements

Install build dependencies on Mint:

```bash
sudo apt install curl build-essential libwebkit2gtk-4.1-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libssl-dev libx11-dev libxfixes-dev patchelf pkg-config
```

Also install Rust and Node.js 20+.

## Development

```bash
cd /home/matt/Documents/Projects/ClipnPaste
npm install
npm run tauri dev
```

For a local production install:

```bash
./scripts/build-release.sh
./scripts/install-local.sh
```

## Install from GitHub release (recommended)

Download the latest `ClipnPaste_*_amd64.deb` from [Releases](https://github.com/LinkofHyrule89/ClipnPaste/releases), then:

```bash
sudo apt install ./ClipnPaste_*_amd64.deb
```

Start the app (or log out and back in to pick up autostart):

```bash
clipnpaste &
```

### Hotkeys

| Shortcut | Action |
|----------|--------|
| `Super+V` | Open clipboard history (bottom-right) |
| `Super+Shift+S` | Open snipping toolbar |

On Cinnamon, ClipnPaste registers these under **System Settings → Keyboard → Shortcuts → Custom Shortcuts**. Mint Menu can stay on Super.

### Data location

All app data is stored locally:

- Clipboard database: `~/.local/share/clipnpaste/history.db`
- App IPC socket: `~/.local/share/clipnpaste/ipc.sock`

The UI is bundled inside the app; no internet connection is required at runtime.

## Build Debian package

```bash
./scripts/build-deb.sh
```

The `.deb` package is written to `src-tauri/target/release/bundle/deb/`.

## Mint Menu / Cinnamon shortcuts

On Cinnamon, ClipnPaste registers `Super+V` and `Super+Shift+S` through the desktop keybinding system (the same path used by panel applets), so Mint Menu can keep Super on its own. On other desktops it falls back to X11 chord handling.

If either shortcut still does not work, open **System Settings → Keyboard → Shortcuts → Custom Shortcuts** and confirm the ClipnPaste entries are present and not conflicting.

## Autostart

Copy `assets/clipnpaste.desktop` to `~/.config/autostart/` after installation if you want ClipnPaste to start on login.