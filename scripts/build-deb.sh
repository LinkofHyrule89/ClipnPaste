#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT/scripts/build-env.sh" bash -c "
  cd '$ROOT'
  npm install
  npm run tauri build
"

DEB_DIR="$ROOT/src-tauri/target/release/bundle/deb"
# Prefer newest mtime so older debs in the same folder are not reported.
DEB_FILE="$(find "$DEB_DIR" -maxdepth 1 -name '*.deb' -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -1 | cut -d' ' -f2-)"

if [[ -z "$DEB_FILE" || ! -f "$DEB_FILE" ]]; then
  echo "No .deb package found in $DEB_DIR"
  exit 1
fi

echo "Debian package: $DEB_FILE"
dpkg-deb -I "$DEB_FILE" | sed -n '1,15p'