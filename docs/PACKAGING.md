# Packaging and release

## What ships

| Artifact | Size | Audience |
| --- | --- | --- |
| `.deb` | ~5 MB | Ubuntu 24.04+, Debian 13+. **Recommended.** |
| `.AppImage` | ~90 MB | Every other distribution; no installation |

The binary itself is ~11 MB. The `.deb` is small because it depends on the
system WebKitGTK; the AppImage is large because it bundles GTK and its
support libraries so it can run anywhere.

Release profile: `lto = true`, `codegen-units = 1`, `opt-level = "s"`,
`panic = "abort"`, `strip = true` — optimised for size and startup rather
than raw throughput, which is the right trade for a UI process.

## Building

```bash
./scripts/setup-deps.sh     # once — installs the WebKitGTK toolchain
npm install
./scripts/build-linux.sh    # typecheck, tests, then both bundles
```

Artifacts land in `src-tauri/target/release/bundle/{deb,appimage}/`.

To build one target: `npx tauri build --bundles deb`.

### Two build-environment gotchas

**Stale UI after `cargo build`.** With the `custom-protocol` feature (the
default for a release build), `generate_context!` **embeds** `dist/` into the
binary at compile time. Rebuilding the frontend alone therefore changes
nothing about the installed app. Either use `npm run app:dev`, which serves
from the Vite dev server, or remember to re-run `cargo build` after
`npm run build`. This is easy to lose an hour to.

**AppImage needs FUSE to *build*.** `linuxdeploy` is itself an AppImage, so
on Ubuntu 24.04 (which dropped `libfuse2`) the bundler fails with
`failed to run linuxdeploy`. Either install `libfuse2t64`, or build with:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 npx tauri build --bundles appimage
```

**The AppImage's GTK plugin reads `pkg-config`.** `linuxdeploy-plugin-gtk`
locates GTK's runtime modules through `pkg-config --variable=libdir
gtk+-3.0`. If you build against a relocated sysroot rather than the
distribution's own `-dev` packages, libdir points somewhere without those
modules and the plugin fails (or, worse, copies them to a path that will not
exist on the user's machine). Install the `-dev` packages normally —
`scripts/setup-deps.sh` — and this does not arise.

## Installing

```bash
# Debian / Ubuntu
sudo apt install ./OpenClaude\ Desktop_0.1.0_amd64.deb

# AppImage
chmod +x OpenClaude\ Desktop_0.1.0_amd64.AppImage
./OpenClaude\ Desktop_0.1.0_amd64.AppImage
# if it complains about FUSE:
sudo apt install libfuse2t64
# or, without installing anything:
./OpenClaude\ Desktop_0.1.0_amd64.AppImage --appimage-extract-and-run
```

## Installing without root

`scripts/install-user.sh` unpacks the `.deb` into `~/.local` and registers it
through the XDG directories. Useful on a locked-down machine, and for testing
a build without touching the system. `--uninstall` reverses it, leaving your
conversations and settings in place.

It clears previously installed icons before copying: the set of sizes can
change between versions, and a leftover file in an old directory would keep
being picked up by the icon theme.

## Desktop integration

The `.deb` installs:

- `/usr/bin/openclaude-desktop`
- `/usr/share/applications/dev.openclaude.desktop.desktop`
- `/usr/share/icons/hicolor/{16x16,24x24,32x32,48x48,64x64,128x128,256x256,512x512}/apps/openclaude.png`
- `/usr/share/metainfo/dev.openclaude.desktop.metainfo.xml` — AppStream data,
  so GNOME Software and Discover show a proper entry

Declared runtime dependencies: `libwebkit2gtk-4.1-0`, `libgtk-3-0`,
`libayatana-appindicator3-1`. apt refuses to install where WebKitGTK 4.1 is
unavailable, which is a much better failure than a blank window.

## Verifying a build

```bash
dpkg -c "src-tauri/target/release/bundle/deb/OpenClaude Desktop_0.1.0_amd64.deb"
dpkg -I "src-tauri/target/release/bundle/deb/OpenClaude Desktop_0.1.0_amd64.deb"
desktop-file-validate /usr/share/applications/dev.openclaude.desktop.desktop
ldd src-tauri/target/release/openclaude | grep -i webkit
```

## Releasing

1. Bump the version in `package.json`, `src-tauri/Cargo.toml` and
   `src-tauri/tauri.conf.json` — all three must match.
2. Add a `<release>` entry to `assets/dev.openclaude.desktop.metainfo.xml`.
3. Update `CHANGELOG.md`.
4. `./scripts/build-linux.sh` and smoke-test both artifacts.
5. Tag `v0.1.0` and push. `.github/workflows/release.yml` builds on a clean
   Ubuntu runner and attaches the artifacts to a draft release.
6. Publish the draft once the artifacts are checked.

## Updates

Deliberately **not** enabled by default. The Tauri updater is wired for later
use but disabled, because an auto-updater is a code-execution channel and one
should not be switched on without signing keys and a published policy.

When enabled it will require a minisign keypair (`tauri signer generate`),
with the public key in `tauri.conf.json` and the private key held only in CI
secrets. Until then, updates come from apt or from downloading a new
AppImage.

## Future targets

- **Flatpak** — the right answer for Fedora/Arch/openSUSE, and gives portal
  integration for Phase 4 screenshots. Needs a manifest and a Flathub
  submission.
- **RPM** — a one-line addition to `bundle.targets`; add when someone asks.
- **AUR** — a `PKGBUILD` building from source.
- **`aarch64`** — the build is architecture-agnostic; it needs a runner.
