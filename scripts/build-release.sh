#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

"$ROOT/scripts/build-env.sh" bash -c "
  cd '$ROOT'
  npm run build
  cd src-tauri
  cargo build --release --features custom-protocol
"

echo "Release binary: $ROOT/src-tauri/target/release/clipnpaste"