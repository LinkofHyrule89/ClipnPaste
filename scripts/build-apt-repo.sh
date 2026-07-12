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

cat > "$OUT/index.html" << 'HTML'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>ClipnPaste apt repository</title>
  <style>
    :root { color-scheme: dark; font-family: system-ui, sans-serif; }
    body { max-width: 40rem; margin: 2rem auto; padding: 0 1rem; line-height: 1.5; background: #111; color: #eee; }
    code, pre { background: #1e1e1e; border-radius: 6px; }
    code { padding: 0.15em 0.4em; }
    pre { padding: 1rem; overflow-x: auto; }
    a { color: #38bdf8; }
    h1 { font-size: 1.5rem; }
  </style>
</head>
<body>
  <h1>ClipnPaste apt repository</h1>
  <p>Unofficial repo hosted on GitHub Pages for Linux Mint / Debian / Ubuntu (amd64).</p>
  <h2>Quick install</h2>
  <pre>curl -fsSL https://linkofhyrule89.github.io/ClipnPaste/install.sh | sudo bash</pre>
  <h2>Manual</h2>
  <pre>echo 'deb [arch=amd64 trusted=yes] https://linkofhyrule89.github.io/ClipnPaste stable main' \
  | sudo tee /etc/apt/sources.list.d/clipnpaste.list
sudo apt update
sudo apt install clipn-paste</pre>
  <p>Package name is <code>clipn-paste</code> (binary: <code>clipnpaste</code>).</p>
  <h2>Uninstall</h2>
  <pre>pkill -x clipnpaste 2>/dev/null || true
sudo apt remove clipn-paste
# optional: remove config/data leftovers from the package
sudo apt purge clipn-paste
# remove this apt source
sudo rm -f /etc/apt/sources.list.d/clipnpaste.list
sudo apt update</pre>
  <p><a href="https://github.com/LinkofHyrule89/ClipnPaste">Source &amp; releases</a></p>
</body>
</html>
HTML

# No Jekyll so apt paths with underscores/dots are served as-is
touch "$OUT/.nojekyll"

echo
echo "Apt repo ready: $OUT"
echo "Packages:"
grep -E '^(Package|Version|Filename):' "$OUT/dists/stable/main/binary-amd64/Packages" || true
