#!/usr/bin/env bash
#
# Install the system packages needed to build OpenClaude Desktop.
# Ubuntu / Debian. Run once, before the first build.
set -euo pipefail

PACKAGES=(
  # Tauri v2 on Linux builds against WebKitGTK 4.1 and GTK 3.
  libwebkit2gtk-4.1-dev
  libjavascriptcoregtk-4.1-dev
  libgtk-3-dev
  libsoup-3.0-dev
  libglib2.0-dev
  # System tray (used from Phase 2 onwards).
  libayatana-appindicator3-dev
  # SVG icon rendering used by the bundler.
  librsvg2-dev
  # Credential storage via the Secret Service API.
  libsecret-1-dev
  # Toolchain and packaging.
  build-essential curl wget file pkg-config patchelf desktop-file-utils
  # Needed to *run* AppImages on Ubuntu 24.04.
  libfuse2t64
)

echo "Installing build dependencies (requires sudo)…"
sudo apt-get update
sudo apt-get install -y "${PACKAGES[@]}"

echo
echo "Checking pkg-config can see everything:"
missing=0
for mod in webkit2gtk-4.1 javascriptcoregtk-4.1 gtk+-3.0 libsoup-3.0 glib-2.0 libsecret-1; do
  if version=$(pkg-config --modversion "$mod" 2>/dev/null); then
    printf '  %-26s %s\n' "$mod" "$version"
  else
    printf '  %-26s MISSING\n' "$mod"
    missing=1
  fi
done

if [ "$missing" -ne 0 ]; then
  echo
  echo "Some modules are still missing. Check the output of apt above." >&2
  exit 1
fi

echo
echo "Done. Next:  npm install  &&  npm run app:dev"
