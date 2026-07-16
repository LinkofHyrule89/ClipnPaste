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
