#!/usr/bin/env bash
#
# Build the release bundles (.deb and AppImage) and report where they landed.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Checking the toolchain"
command -v cargo >/dev/null || { echo "cargo not found — install Rust from https://rustup.rs" >&2; exit 1; }
command -v node  >/dev/null || { echo "node not found — Node 18.18+ is required" >&2; exit 1; }
pkg-config --exists webkit2gtk-4.1 || {
  echo "webkit2gtk-4.1 not found — run scripts/setup-deps.sh first" >&2
  exit 1
}

echo "==> Installing npm dependencies"
npm ci 2>/dev/null || npm install

echo "==> Checks"
npm run typecheck
npm test
cargo test --manifest-path src-tauri/Cargo.toml

echo "==> Bundling"
npx tauri build --bundles deb appimage

OUT="src-tauri/target/release/bundle"
echo
echo "==> Artifacts"
find "$OUT" -maxdepth 2 -type f \( -name '*.deb' -o -name '*.AppImage' \) -print0 |
  while IFS= read -r -d '' f; do
    printf '  %-72s %s\n' "$f" "$(du -h "$f" | cut -f1)"
  done

cat <<'EOF'

Install the .deb with:
    sudo apt install ./src-tauri/target/release/bundle/deb/*.deb

Or run the AppImage directly (needs libfuse2t64 on Ubuntu 24.04):
    chmod +x src-tauri/target/release/bundle/appimage/*.AppImage
    ./src-tauri/target/release/bundle/appimage/*.AppImage
EOF
