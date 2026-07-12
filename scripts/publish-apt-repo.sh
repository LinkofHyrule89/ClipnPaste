#!/usr/bin/env bash
# Download .deb assets from GitHub releases, build the apt repo, push to gh-pages.
#
# Usage (from repo root, with gh auth):
#   ./scripts/publish-apt-repo.sh
#
# Optional: only use local debs
#   DEB_DIR=./path/to/debs ./scripts/publish-apt-repo.sh --local-only
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

REPO="${GITHUB_REPOSITORY:-LinkofHyrule89/ClipnPaste}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

DEB_COLLECT="$WORK/debs"
REPO_OUT="$WORK/apt"
mkdir -p "$DEB_COLLECT"

LOCAL_ONLY=0
if [[ "${1:-}" == "--local-only" ]]; then
  LOCAL_ONLY=1
fi

if [[ -n "${DEB_DIR:-}" ]]; then
  cp -f "$DEB_DIR"/*.deb "$DEB_COLLECT/" 2>/dev/null || true
fi

# Always pick up a freshly built local deb if present
LOCAL_BUNDLE="$ROOT/src-tauri/target/release/bundle/deb"
if [[ -d "$LOCAL_BUNDLE" ]]; then
  cp -f "$LOCAL_BUNDLE"/*.deb "$DEB_COLLECT/" 2>/dev/null || true
fi

if [[ "$LOCAL_ONLY" -eq 0 ]]; then
  if ! command -v gh >/dev/null; then
    echo "gh CLI required to pull release assets (or use --local-only)" >&2
    exit 1
  fi
  echo "Downloading .deb assets from GitHub releases…"
  # List tags and download each matching asset (ignore releases without debs)
  while read -r tag; do
    [[ -z "$tag" ]] && continue
    gh release download "$tag" \
      --repo "$REPO" \
      --pattern "*.deb" \
      --dir "$DEB_COLLECT" \
      --clobber 2>/dev/null || true
  done < <(gh release list --repo "$REPO" --limit 50 --json tagName --jq '.[].tagName')
fi

count="$(find "$DEB_COLLECT" -maxdepth 1 -name '*.deb' | wc -l | tr -d ' ')"
if [[ "$count" -eq 0 ]]; then
  echo "No .deb files collected" >&2
  exit 1
fi
echo "Collected $count package(s)"

"$ROOT/scripts/build-apt-repo.sh" "$DEB_COLLECT" "$REPO_OUT"

# Deploy to gh-pages
if [[ "${SKIP_PUSH:-0}" == "1" ]]; then
  echo "SKIP_PUSH=1 — repo built at $REPO_OUT (not pushed)"
  # Keep a copy for inspection when testing
  rm -rf /tmp/clipnpaste-apt-preview
  cp -a "$REPO_OUT" /tmp/clipnpaste-apt-preview
  echo "Preview: /tmp/clipnpaste-apt-preview"
  exit 0
fi

echo "Publishing to gh-pages…"
PAGES_DIR="$WORK/gh-pages"
if git ls-remote --exit-code --heads origin gh-pages >/dev/null 2>&1; then
  git clone --depth 1 --branch gh-pages "https://github.com/${REPO}.git" "$PAGES_DIR"
  # Remove old tree but keep .git
  find "$PAGES_DIR" -mindepth 1 -maxdepth 1 ! -name '.git' -exec rm -rf {} +
else
  mkdir -p "$PAGES_DIR"
  git -C "$PAGES_DIR" init
  git -C "$PAGES_DIR" checkout -b gh-pages
  git -C "$PAGES_DIR" remote add origin "https://github.com/${REPO}.git"
fi

cp -a "$REPO_OUT"/. "$PAGES_DIR"/
git -C "$PAGES_DIR" add -A
if git -C "$PAGES_DIR" diff --cached --quiet; then
  echo "No changes to publish"
else
  git -C "$PAGES_DIR" \
    -c user.name="ClipnPaste bot" \
    -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
    commit -m "Update apt repository"
  git -C "$PAGES_DIR" push -u origin gh-pages
fi

# Ensure GitHub Pages serves the gh-pages branch (ignore if already set).
if command -v gh >/dev/null; then
  echo "Configuring GitHub Pages (gh-pages /)…"
  gh api -X POST "repos/${REPO}/pages" --input - <<'EOF' >/dev/null 2>&1 || true
{"build_type":"legacy","source":{"branch":"gh-pages","path":"/"}}
EOF
  gh api -X PUT "repos/${REPO}/pages" --input - <<'EOF' >/dev/null 2>&1 || true
{"build_type":"legacy","source":{"branch":"gh-pages","path":"/"}}
EOF
fi

echo
echo "Done. After Pages finishes deploying (often ~1 min):"
echo "  https://linkofhyrule89.github.io/ClipnPaste/"
echo "  curl -fsSL https://linkofhyrule89.github.io/ClipnPaste/install.sh | sudo bash"
