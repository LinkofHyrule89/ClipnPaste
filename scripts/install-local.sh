#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="$ROOT/src-tauri/target/release/clipnpaste"
INSTALL_BIN="$HOME/.local/bin/clipnpaste"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"
AUTOSTART_DIR="$HOME/.config/autostart"

if [[ ! -x "$BINARY" ]]; then
  echo "Release binary not found at $BINARY"
  echo "Run: ./scripts/build-env.sh npm run tauri build"
  exit 1
fi

mkdir -p "$HOME/.local/bin" "$DESKTOP_DIR" "$ICON_DIR" "$AUTOSTART_DIR"
install -m 755 "$BINARY" "$INSTALL_BIN"
install -m 644 "$ROOT/assets/clipnpaste.desktop" "$DESKTOP_DIR/clipnpaste.desktop"
install -m 644 "$ROOT/src-tauri/icons/128x128.png" "$ICON_DIR/clipnpaste.png"
sed "s|^Exec=clipnpaste$|Exec=$INSTALL_BIN|" "$ROOT/assets/clipnpaste.desktop" > "$AUTOSTART_DIR/clipnpaste.desktop"

echo "Installed ClipnPaste to $INSTALL_BIN"
echo "Desktop entry: $DESKTOP_DIR/clipnpaste.desktop"
echo "Autostart entry: $AUTOSTART_DIR/clipnpaste.desktop"