#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> Installing system dependencies (requires sudo)..."
sudo apt-get update
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
  build-essential curl libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev libssl-dev libx11-dev \
  libxfixes-dev libdbus-1-dev patchelf pkg-config xclip

if [[ -f "$HOME/.cargo/env" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi

if [[ -f "$HOME/.nvm/nvm.sh" ]]; then
  # shellcheck disable=SC1091
  source "$HOME/.nvm/nvm.sh"
fi

echo "==> Building ClipnPaste..."
npm install
npm run tauri build

DEB="$ROOT/src-tauri/target/release/bundle/deb/"*.deb
echo "==> Installing $DEB (requires sudo)..."
sudo dpkg -i "$DEB"
sudo apt-get install -f -y

echo ""
echo "ClipnPaste installed system-wide."
echo "  Run: clipnpaste"
echo "  Hotkeys: Super+V (clipboard), Super+Shift+S (snip)"
echo ""
echo "If hotkeys conflict with Cinnamon, remove them in:"
echo "  System Settings -> Keyboard -> Shortcuts"