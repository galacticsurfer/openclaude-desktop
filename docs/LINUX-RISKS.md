# Linux-specific risks

Things that will actually go wrong on a user's machine, and what the app does
about them.

## WebKitGTK version skew

**Risk.** Unlike Electron, Tauri uses the system WebView. Ubuntu 24.04 ships
WebKitGTK 2.4x–2.5x; older distributions ship considerably older. A CSS or JS
feature that works here may not work there, and the app cannot ship a fix.

**Mitigation.** The build targets ES2022 and avoids very recent CSS. The
`.deb` declares `libwebkit2gtk-4.1-0` so apt refuses to install on a system
without it. The AppImage bundles GTK/WebKit support libraries via
`linuxdeploy-plugin-gtk`.

**Residual.** Distributions still carrying only webkit2gtk **4.0** (Debian 11,
Ubuntu 22.04) cannot run this build. That is a Tauri 2 constraint, not a
choice, and it is why Ubuntu 24.04+ is the stated target.

## Compositing crashes and blank windows

**Risk.** WebKitGTK's accelerated compositing interacts badly with some
drivers — notably older NVIDIA under X11, and some VM/remote-desktop stacks.
The symptom is a white or black window with a healthy process.

**Mitigation.** Documented in the README:

```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 openclaude
```

This is also what the development harness uses.

## No Secret Service available

**Risk.** Headless sessions, minimal window managers, or a broken
`gnome-keyring` mean no credential store. Naively, the app would either crash
or silently write the key somewhere unsafe.

**Mitigation.** `secrets::keyring_available()` probes once. On failure the app
falls back to memory for the session and says so in the sidebar badge and in
Settings → Claude. Nothing is written to disk. See
[SECURITY-MODEL.md](SECURITY-MODEL.md) for why an encrypted-file fallback was
rejected.

## Wayland vs X11

**Risk.** Window positioning is not available to clients under Wayland, so
"restore window position" cannot be fully honoured. Global shortcuts and
screenshot capture (Phase 4) require XDG portals rather than direct X calls.

**Mitigation.** Size and maximised state are restored on both; position is
best-effort. Phase 4 features are planned against portals from the start.

## AppImage and FUSE

**Risk.** Ubuntu 24.04 dropped `libfuse2`, so AppImage v2 files fail with a
confusing message about FUSE.

**Mitigation.** The README gives both fixes: install `libfuse2t64`, or run
with `--appimage-extract-and-run`. The `.deb` is recommended for Ubuntu users
precisely because it avoids this.

## Tray icons

**Risk.** GNOME has no tray by default; it needs the AppIndicator extension.
KDE and others work out of the box. A tray-only app would be unusable on
stock GNOME.

**Mitigation.** Tray is opt-in and never required — closing the window always
quits unless the user explicitly chose otherwise. The setting is disabled in
the UI until the feature ships in Phase 2.

## Font availability

**Risk.** Bundling proprietary fonts is a licensing problem; assuming a
monospace font exists is a rendering problem.

**Mitigation.** No fonts are bundled. The stacks list fonts commonly present
on Linux (Ubuntu, Cantarell, Noto, DejaVu, Liberation) and end in generic
`sans-serif` / `monospace`, which always resolve through fontconfig.

## Clock and timezone

**Risk.** Sidebar grouping by elapsed hours would label something from 23:00
last night as "Today" at 00:30.

**Mitigation.** `dateGroup` buckets by **local calendar day**, and is tested
against exactly that case.

## Filesystem surprises

**Risk.** `~/.local/share` on NFS breaks SQLite WAL locking. Case-insensitive
mounts break content-addressed blob paths.

**Mitigation.** `busy_timeout` is set, and `OPENCLAUDE_DB` lets the user
relocate the database to local storage. `PRAGMA integrity_check` is exposed in
Settings → Advanced.

## Distribution packaging

**Risk.** The `.deb` targets Debian/Ubuntu only. Fedora, Arch and openSUSE
users need the AppImage or a source build.

**Mitigation.** The AppImage is the portable path for Phase 1. Flatpak is the
right long-term answer for those distributions and is on the roadmap; RPM is
a straightforward addition to the bundler config when someone needs it.
