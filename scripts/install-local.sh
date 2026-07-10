#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="$ROOT/src-tauri/target/release/clipnpaste"
CLI_BINARY="$ROOT/src-tauri/target/release/clipnpaste-cli"
INSTALL_BIN="$HOME/.local/bin/clipnpaste"
INSTALL_CLI="$HOME/.local/bin/clipnpaste-cli"
DESKTOP_DIR="$HOME/.local/share/applications"
ICON_ROOT="$HOME/.local/share/icons/hicolor"
AUTOSTART_DIR="$HOME/.config/autostart"

if [[ ! -x "$BINARY" ]]; then
  echo "Release binary not found at $BINARY"
  echo "Run: ./scripts/build-release.sh"
  exit 1
fi

mkdir -p "$HOME/.local/bin" "$DESKTOP_DIR" "$AUTOSTART_DIR"

install -m 755 "$BINARY" "$INSTALL_BIN"
if [[ -x "$CLI_BINARY" ]]; then
  install -m 755 "$CLI_BINARY" "$INSTALL_CLI"
fi

# Freedesktop icon theme sizes (Mint menu looks for 24/48/64 among others)
for size in 16 22 24 32 48 64 96 128 256 512; do
  src="$ROOT/src-tauri/icons/hicolor/${size}x${size}/apps/clipnpaste.png"
  if [[ -f "$src" ]]; then
    dest_dir="$ICON_ROOT/${size}x${size}/apps"
    mkdir -p "$dest_dir"
    install -m 644 "$src" "$dest_dir/clipnpaste.png"
  fi
done

# Desktop entry with absolute Exec path for local installs
sed "s|^Exec=clipnpaste$|Exec=$INSTALL_BIN|" "$ROOT/assets/clipnpaste.desktop" \
  > "$DESKTOP_DIR/clipnpaste.desktop"
chmod 644 "$DESKTOP_DIR/clipnpaste.desktop"
sed "s|^Exec=clipnpaste$|Exec=$INSTALL_BIN|" "$ROOT/assets/clipnpaste.desktop" \
  > "$AUTOSTART_DIR/clipnpaste.desktop"
chmod 644 "$AUTOSTART_DIR/clipnpaste.desktop"

# Refresh caches so the Mint menu picks up Icon=clipnpaste
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$ICON_ROOT" 2>/dev/null || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
fi

echo "Installed ClipnPaste to $INSTALL_BIN"
echo "Desktop entry: $DESKTOP_DIR/clipnpaste.desktop"
echo "Autostart entry: $AUTOSTART_DIR/clipnpaste.desktop"
echo "Icons: $ICON_ROOT/*/apps/clipnpaste.png"
