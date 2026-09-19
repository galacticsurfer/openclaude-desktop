#!/usr/bin/env bash
#
# Start the app in development mode with hot reload.
#
# Note: `tauri dev` serves the UI from the Vite dev server, so frontend edits
# reload instantly. A plain `cargo build` instead *embeds* the contents of
# dist/ at compile time — if you build that way, re-run `npm run build`
# followed by `cargo build`, or the binary will keep serving a stale UI.
set -euo pipefail
cd "$(dirname "$0")/.."
export OPENCLAUDE_DEV=1
exec npx tauri dev "$@"
