# Snap package foundation

`snapcraft.yaml` creates a strict-confinement `capture-vault` snap from the
current Tauri source. It builds the normal Tauri Debian bundle first, then
extracts that bundle into the snap. This preserves Tauri's resource layout,
including the bundled OCR models and desktop launcher.

## Local package check

Install a current Snapcraft, then run this from the repository root:

```bash
snapcraft pack
```

Use Snapcraft's normal isolated build provider (LXD on Linux) for a release
candidate. Do not use `--destructive-mode` on a developer workstation: the
build must use the `core24` environment and installs the Node, Rust, and GNOME
SDK build snaps declared by the manifest.

Use a disposable test machine or VM to install the resulting snap:

```bash
sudo snap install --dangerous capture-vault_0.2.0_amd64.snap
capture-vault
```

Before any Store upload, test all of the following with the strict snap:

- area and full-screen capture through the XDG Desktop Portal on Ubuntu Wayland
  and X11;
- global shortcuts, image copy, and drag-out;
- default storage and a user-selected folder under `$HOME`;
- explicit opt-in to the optional loopback vision endpoint; and
- launch, search, OCR, and bundled-model loading after a clean install.

The GNOME extension supplies the standard desktop, Wayland, X11, OpenGL, and
settings interfaces, plus the GTK and WebKit runtime through its content snap.
Do not add `libgtk-3-0` or `libwebkit2gtk-4.1-0` as staged packages unless a
future product requirement needs a deliberately bundled runtime: doing so
duplicates hundreds of files, bypasses the extension's runtime layouts, and
creates avoidable linter warnings. The manifest adds `home` for a
user-selected library location and `network` only for the optional local
vision endpoint. `network` is an interface-level permission; CaptureVault
keeps vision disabled until `CAPTURE_VAULT_ENABLE_VISION=1` is set, rejects
non-loopback endpoints and redirects, and cannot control what a separately run
local service does after it receives an opted-in capture.

## Store release path

This project is **not registered or published in the Snap Store yet**. Once
the package checks pass:

1. Register `capture-vault` with the publisher's Snap Store account.
2. Create Store assets using synthetic screenshots and concise Linux-preview
   copy, including the bundled OCR-model attribution from
   `src-tauri/resources/ocr/NOTICE.md`.
3. Upload a tested build to `candidate` or `beta` first, then validate it on
   Wayland and X11 before promoting it to `stable`.
4. Add a tag-triggered Store upload workflow only after credentials are stored
   as repository secrets and the manual release process is proven.

No login, registration, upload, or channel promotion is performed by this
repository configuration.
