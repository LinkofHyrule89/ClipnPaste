#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT/scripts/build-env.sh" bash -c "
  cd '$ROOT'
  npm install
  npm run tauri build
"

DEB_DIR="$ROOT/src-tauri/target/release/bundle/deb"
DEB_FILE="$(find "$DEB_DIR" -maxdepth 1 -name '*.deb' -print -quit)"

if [[ -z "$DEB_FILE" ]]; then
  echo "No .deb package found in $DEB_DIR"
  exit 1
fi

echo "Debian package: $DEB_FILE"