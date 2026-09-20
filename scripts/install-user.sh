#!/usr/bin/env bash
#
# Install OpenClaude Desktop into ~/.local, without root.
#
# The .deb's only dependencies are WebKitGTK, GTK 3 and libayatana-appindicator,
# which a desktop system already has. So rather than needing `sudo apt install`,
# we can unpack the same package into the user prefix and register it with the
# desktop the normal XDG way.
#
#   ./scripts/install-user.sh [path/to/.deb]
#   ./scripts/install-user.sh --uninstall
set -euo pipefail

PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$PREFIX/applications"
ICON_DIR="$PREFIX/icons/hicolor"
META_DIR="$PREFIX/metainfo"

BIN_NAME="openclaude"
DESKTOP_FILE="$APP_DIR/openclaude-desktop.desktop"

uninstall() {
    echo "Removing OpenClaude Desktop from $HOME/.local …"
    rm -f "$BIN_DIR/$BIN_NAME" "$DESKTOP_FILE" "$META_DIR/dev.openclaude.desktop.metainfo.xml"
    find "$ICON_DIR" -name "$BIN_NAME.png" -delete 2>/dev/null || true
    refresh
    cat <<EOF

Removed. Your conversations and settings were left alone:
    ${XDG_DATA_HOME:-$HOME/.local/share}/openclaude
    ${XDG_CONFIG_HOME:-$HOME/.config}/openclaude
Delete those by hand if you want them gone. Your API key lives in the system
keyring; remove it from Settings before uninstalling, or with a keyring tool.
EOF
    exit 0
}

refresh() {
    command -v update-desktop-database >/dev/null && update-desktop-database "$APP_DIR" 2>/dev/null || true
    command -v gtk-update-icon-cache   >/dev/null && gtk-update-icon-cache -qtf "$ICON_DIR" 2>/dev/null || true
}

[ "${1:-}" = "--uninstall" ] && uninstall

cd "$(dirname "$0")/.."
DEB="${1:-}"
if [ -z "$DEB" ]; then
    DEB=$(find src-tauri/target/release/bundle/deb -name '*.deb' -print -quit 2>/dev/null || true)
fi
if [ -z "$DEB" ] || [ ! -f "$DEB" ]; then
    echo "No .deb found. Build one first:  ./scripts/build-linux.sh" >&2
    exit 1
fi

echo "Installing from: $DEB"

TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
dpkg -x "$DEB" "$TMP"

# Ask the dynamic linker about the actual binary rather than guessing from a
# package list — `ldconfig` is in /usr/sbin and is usually not on a user PATH,
# so probing with it produces false alarms.
if command -v ldd >/dev/null; then
    missing=$(ldd "$TMP/usr/bin/$BIN_NAME" 2>/dev/null | awk '/not found/ {print "  " $1}')
    if [ -n "$missing" ]; then
        echo "Warning: these shared libraries are missing:" >&2
        echo "$missing" >&2
        echo "  Install them with: ./scripts/setup-deps.sh" >&2
    fi
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR" "$META_DIR"

install -Dm755 "$TMP/usr/bin/$BIN_NAME" "$BIN_DIR/$BIN_NAME"

# Clear icons from a previous install first: the set of sizes can change
# between versions, and a leftover file in an old directory would keep being
# picked by the icon theme.
find "$ICON_DIR" -name "$BIN_NAME.png" -delete 2>/dev/null || true

while IFS= read -r -d '' icon; do
    rel="${icon#"$TMP"/usr/share/icons/hicolor/}"
    install -Dm644 "$icon" "$ICON_DIR/$rel"
done < <(find "$TMP/usr/share/icons/hicolor" -name '*.png' -print0)

if [ -f "$TMP/usr/share/metainfo/dev.openclaude.desktop.metainfo.xml" ]; then
    install -Dm644 "$TMP/usr/share/metainfo/dev.openclaude.desktop.metainfo.xml" \
        "$META_DIR/dev.openclaude.desktop.metainfo.xml"
fi

# Rewrite Exec/TryExec to the absolute path: a desktop session does not always
# have ~/.local/bin on PATH, even when an interactive shell does.
src_desktop=$(find "$TMP/usr/share/applications" -name '*.desktop' -print -quit)
sed -e "s|^Exec=.*|Exec=$BIN_DIR/$BIN_NAME %U|" \
    -e "s|^Icon=.*|Icon=$BIN_NAME|" \
    "$src_desktop" > "$DESKTOP_FILE"
grep -q '^TryExec=' "$DESKTOP_FILE" || sed -i "/^Exec=/i TryExec=$BIN_DIR/$BIN_NAME" "$DESKTOP_FILE"
chmod 644 "$DESKTOP_FILE"

command -v desktop-file-validate >/dev/null && desktop-file-validate "$DESKTOP_FILE"

refresh

cat <<EOF

Installed.

  binary    $BIN_DIR/$BIN_NAME
  launcher  $DESKTOP_FILE

Run it from your application menu ("OpenClaude Desktop"), or from a terminal:

    $BIN_NAME

If the window comes up blank, your GPU driver and WebKitGTK's compositing are
not getting along — try:

    WEBKIT_DISABLE_COMPOSITING_MODE=1 $BIN_NAME

To remove it later:  ./scripts/install-user.sh --uninstall
EOF
