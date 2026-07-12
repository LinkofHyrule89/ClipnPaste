#!/usr/bin/env bash
# Build a static Debian apt repository from one or more .deb files.
#
# Usage:
#   ./scripts/build-apt-repo.sh <deb-dir> <output-dir>
#
# Layout (served at https://<user>.github.io/ClipnPaste/):
#   pool/main/.../*.deb
#   dists/stable/main/binary-amd64/Packages{,.gz}
#   dists/stable/Release
#   install.sh, index.html
set -euo pipefail

DEB_DIR="${1:?usage: build-apt-repo.sh <deb-dir> <output-dir>}"
OUT="${2:?usage: build-apt-repo.sh <deb-dir> <output-dir>}"

if ! command -v dpkg-scanpackages >/dev/null; then
  echo "dpkg-scanpackages not found (apt install dpkg-dev)" >&2
  exit 1
fi

shopt -s nullglob
DEBS=("$DEB_DIR"/*.deb)
if [[ ${#DEBS[@]} -eq 0 ]]; then
  echo "No .deb files in $DEB_DIR" >&2
  exit 1
fi

rm -rf "$OUT"
mkdir -p "$OUT/pool/main" "$OUT/dists/stable/main/binary-amd64"

for deb in "${DEBS[@]}"; do
  base="$(basename "$deb")"
  # Keep flat pool so Packages Filename: paths stay simple
  cp -f "$deb" "$OUT/pool/main/$base"
  echo "  + $base"
done

(
  cd "$OUT"
  # Paths in Packages are relative to the repo root (where the deb line points).
  dpkg-scanpackages -m pool/main /dev/null > dists/stable/main/binary-amd64/Packages
  gzip -9c dists/stable/main/binary-amd64/Packages > dists/stable/main/binary-amd64/Packages.gz
)

# Release metadata (unsigned; clients use trusted=yes — see install.sh)
DATE="$(date -Ru)"
{
  echo "Origin: ClipnPaste"
  echo "Label: ClipnPaste"
  echo "Suite: stable"
  echo "Codename: stable"
  echo "Architectures: amd64"
  echo "Components: main"
  echo "Description: ClipnPaste unofficial apt repository (GitHub Pages)"
  echo "Date: $DATE"
} > "$OUT/dists/stable/Release"

# Checksums for Release file
(
  cd "$OUT/dists/stable"
  {
    echo "MD5Sum:"
    for f in main/binary-amd64/Packages main/binary-amd64/Packages.gz; do
      size=$(wc -c < "$f" | tr -d ' ')
      sum=$(md5sum "$f" | awk '{print $1}')
      printf " %s %8s %s\n" "$sum" "$size" "$f"
    done
    echo "SHA256:"
    for f in main/binary-amd64/Packages main/binary-amd64/Packages.gz; do
      size=$(wc -c < "$f" | tr -d ' ')
      sum=$(sha256sum "$f" | awk '{print $1}')
      printf " %s %8s %s\n" "$sum" "$size" "$f"
    done
  } >> Release
)

# One-shot installer for users
cat > "$OUT/install.sh" << 'INSTALL'
#!/usr/bin/env bash
# Add the ClipnPaste apt repository and install the package.
set -euo pipefail

if [[ "$(id -u)" -ne 0 ]]; then
  echo "Re-run with sudo: curl -fsSL https://linkofhyrule89.github.io/ClipnPaste/install.sh | sudo bash" >&2
  exit 1
fi

REPO_URL="https://linkofhyrule89.github.io/ClipnPaste"
LIST="/etc/apt/sources.list.d/clipnpaste.list"

# trusted=yes: repo is unsigned (no GPG key to manage). Fine for a personal
# GitHub Pages repo; MITM would require compromising GitHub Pages / DNS.
echo "deb [arch=amd64 trusted=yes] ${REPO_URL} stable main" > "$LIST"
echo "Wrote $LIST"

apt-get update -o Dir::Etc::sourcelist="$LIST" -o Dir::Etc::sourceparts="-" -o APT::Get::List-Cleanup="0" || apt-get update
apt-get install -y clipn-paste

echo
echo "Installed. Start with: clipnpaste &"
echo "Or log out/in for autostart."
echo
echo "Uninstall later:"
echo "  sudo apt remove clipn-paste"
echo "  sudo rm -f /etc/apt/sources.list.d/clipnpaste.list && sudo apt update"
INSTALL
chmod +x "$OUT/install.sh"

# Social / Discord preview assets (absolute URLs in og: tags)
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -f "$ROOT/docs/social-preview.png" ]]; then
  cp -f "$ROOT/docs/social-preview.png" "$OUT/social-preview.png"
fi
if [[ -f "$ROOT/docs/screenshots/01-history.png" ]]; then
  mkdir -p "$OUT/screenshots"
  cp -f "$ROOT/docs/screenshots/01-history.png" "$OUT/screenshots/" 2>/dev/null || true
  cp -f "$ROOT/docs/screenshots/03-emoji.png" "$OUT/screenshots/" 2>/dev/null || true
fi

cat > "$OUT/index.html" << 'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>ClipnPaste — clipboard history &amp; snipping for Linux</title>
  <meta name="description" content="Windows 11-style clipboard history and snipping tool for Linux Mint and Debian." />
  <meta property="og:title" content="ClipnPaste" />
  <meta property="og:description" content="Windows 11-style clipboard history and snipping for Linux Mint" />
  <meta property="og:image" content="https://linkofhyrule89.github.io/ClipnPaste/social-preview.png" />
  <meta property="og:image:width" content="1280" />
  <meta property="og:image:height" content="640" />
  <meta property="og:url" content="https://linkofhyrule89.github.io/ClipnPaste/" />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="ClipnPaste" />
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content="ClipnPaste" />
  <meta name="twitter:description" content="Windows 11-style clipboard history and snipping for Linux Mint" />
  <meta name="twitter:image" content="https://linkofhyrule89.github.io/ClipnPaste/social-preview.png" />
  <style>
    :root { color-scheme: dark; font-family: system-ui, sans-serif; }
    body { max-width: 42rem; margin: 2rem auto; padding: 0 1rem; line-height: 1.5; background: #111; color: #eee; }
    code, pre { background: #1e1e1e; border-radius: 6px; }
    code { padding: 0.15em 0.4em; }
    pre { padding: 1rem; overflow-x: auto; }
    a { color: #38bdf8; }
    h1 { font-size: 1.75rem; margin-bottom: 0.35rem; }
    .tagline { color: #a3a3a3; margin-top: 0; }
    .shots { display: flex; gap: 0.75rem; flex-wrap: wrap; margin: 1.25rem 0; }
    .shots img { height: 160px; width: auto; border-radius: 8px; border: 1px solid rgba(255,255,255,0.1); }
    h2 { font-size: 1.1rem; margin-top: 1.75rem; }
  </style>
</head>
<body>
  <h1>ClipnPaste</h1>
  <p class="tagline">Windows 11-style clipboard history and snipping for Linux Mint / Debian (amd64).</p>
  <div class="shots">
    <img src="screenshots/01-history.png" alt="Clipboard history" width="113" height="160" />
    <img src="screenshots/03-emoji.png" alt="Emoji picker" width="113" height="160" />
  </div>
  <p><a href="https://github.com/LinkofHyrule89/ClipnPaste">Source on GitHub</a> · package name <code>clipn-paste</code></p>
  <h2>Quick install</h2>
  <pre>curl -fsSL https://linkofhyrule89.github.io/ClipnPaste/install.sh | sudo bash</pre>
  <h2>Manual</h2>
  <pre>echo 'deb [arch=amd64 trusted=yes] https://linkofhyrule89.github.io/ClipnPaste stable main' \
  | sudo tee /etc/apt/sources.list.d/clipnpaste.list
sudo apt update
sudo apt install clipn-paste</pre>
  <h2>Uninstall</h2>
  <pre>pkill -x clipnpaste 2>/dev/null || true
sudo apt remove clipn-paste
sudo apt purge clipn-paste
sudo rm -f /etc/apt/sources.list.d/clipnpaste.list
sudo apt update</pre>
</body>
</html>
HTML

# No Jekyll so apt paths with underscores/dots are served as-is
touch "$OUT/.nojekyll"

echo
echo "Apt repo ready: $OUT"
echo "Packages:"
grep -E '^(Package|Version|Filename):' "$OUT/dists/stable/main/binary-amd64/Packages" || true
