#!/bin/sh
# Installs or updates Mamba in the Applications folder from the latest
# GitHub release, from Terminal:
#
#   curl -fsSL https://raw.githubusercontent.com/bobnautic/Mamba/main/install-macos.sh | sh
#
# Files downloaded by curl carry no quarantine flag, so the app opens
# directly, without the "Open Anyway" step a browser download needs.
set -eu

REPO="bobnautic/Mamba"
API="https://api.github.com/repos/$REPO/releases/latest"

URL=$(curl -fsSL "$API" \
  | grep -o '"browser_download_url": *"[^"]*macos-universal\.app\.tar\.gz"' \
  | head -1 | sed 's/.*"\(https[^"]*\)"/\1/')
if [ -z "$URL" ]; then
  echo "The latest Mamba release has no macOS build yet: https://github.com/$REPO/releases/latest"
  exit 1
fi

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
echo "Downloading $URL"
curl -fL --progress-bar "$URL" -o "$TMP/Mamba.app.tar.gz"
tar -xzf "$TMP/Mamba.app.tar.gz" -C "$TMP"

DEST="/Applications"
if [ ! -w "$DEST" ]; then
  DEST="$HOME/Applications"
  mkdir -p "$DEST"
fi

if pgrep -x Mamba >/dev/null 2>&1; then
  echo "Closing the running Mamba..."
  pkill -x Mamba || true
  sleep 1
fi
rm -rf "$DEST/Mamba.app"
mv "$TMP/Mamba.app" "$DEST/Mamba.app"
echo "Mamba installed in $DEST"
open "$DEST/Mamba.app"
