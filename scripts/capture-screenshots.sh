#!/usr/bin/env bash
# Capture ClipnPaste windows with demo content into docs/screenshots/
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/docs/screenshots"
mkdir -p "$OUT"

export DISPLAY="${DISPLAY:-:0}"

CLIPNPASTE="${CLIPNPASTE:-$HOME/.local/bin/clipnpaste}"
CLI="${CLI:-$HOME/.local/bin/clipnpaste-cli}"

if ! pgrep -x clipnpaste >/dev/null 2>&1; then
  echo "Starting clipnpaste…"
  nohup "$CLIPNPASTE" >/tmp/clipnpaste-screenshot.log 2>&1 &
  sleep 2
fi

# Seed demo data (backs up user DB once)
python3 "$ROOT/scripts/seed-demo-history.py"

# Restart so UI picks up seeded DB
pkill -x clipnpaste 2>/dev/null || true
sleep 0.8
nohup "$CLIPNPASTE" >/tmp/clipnpaste-screenshot.log 2>&1 &
sleep 2

capture_window() {
  local title_pattern="$1"
  local outfile="$2"
  local wid
  sleep 0.5
  wid="$(xdotool search --name "$title_pattern" 2>/dev/null | while read -r w; do
    name="$(xdotool getwindowname "$w" 2>/dev/null || true)"
    if [[ "$name" == *"$title_pattern"* ]]; then
      echo "$w"
      break
    fi
  done | head -1)"
  if [[ -z "$wid" ]]; then
    echo "Window not found for pattern: $title_pattern — full-screen fallback"
    gnome-screenshot -f "$outfile" 2>/dev/null || true
    return
  fi
  xdotool windowactivate --sync "$wid" 2>/dev/null || true
  sleep 0.8
  gnome-screenshot -w -f "$outfile" 2>/dev/null || gnome-screenshot -f "$outfile"
  echo "Wrote $outfile ($(stat -c%s "$outfile" 2>/dev/null || echo '?') bytes)"
}

echo "=== History panel ==="
"$CLI" clipboard 2>/dev/null || true
sleep 1.5
capture_window "Clipboard" "$OUT/01-history.png"

echo "=== Emoji panel ==="
"$CLI" emoji 2>/dev/null || true
sleep 1.5
capture_window "Clipboard" "$OUT/03-emoji.png"

echo "=== Settings ==="
"$CLI" settings 2>/dev/null || true
sleep 1.5
capture_window "ClipnPaste Settings" "$OUT/04-settings.png"

echo "=== Snip toolbar ==="
"$CLI" snip 2>/dev/null || true
sleep 1.2
capture_window "Snip" "$OUT/05-snip-toolbar.png"

# Hide floating panels
xdotool search --name "Clipboard" windowunmap 2>/dev/null || true
xdotool search --name "Snip" windowunmap 2>/dev/null || true
xdotool search --name "Settings" windowunmap 2>/dev/null || true

# Restore user history
python3 "$ROOT/scripts/seed-demo-history.py" restore

pkill -x clipnpaste 2>/dev/null || true
sleep 0.5
nohup "$CLIPNPASTE" >/tmp/clipnpaste.log 2>&1 &

echo "Screenshots in $OUT:"
ls -la "$OUT"
