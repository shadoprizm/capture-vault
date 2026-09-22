# Manual desktop packaging

`package-desktop.yml` is a manually dispatched GitHub Actions workflow for
native package builds. It runs one build each on Ubuntu, Windows, and macOS,
then keeps the generated files as GitHub Actions workflow artifacts for 14
days. Start it from **Actions → Package desktop installers (manual) → Run
workflow**.

The workflow runs the project-pinned Tauri CLI with `npm run tauri build`.
CaptureVault's Tauri configuration uses `bundle.targets: "all"`, so a runner
uploads every bundle Tauri actually produces. That normally includes Linux
AppImage, Debian, and RPM bundles; Windows MSI and NSIS EXE installers; and
macOS `.app` and DMG bundles. The precise formats and CPU architecture remain
the native runner's output rather than a CI assumption.

## Deliberate release boundary

This workflow does **not** create a GitHub Release, tag a version, upload
release assets, or publish to a package marketplace. It is an integration and
packaging check only.

CaptureVault's native screen-capture implementation currently relies on the
Linux XDG Desktop Portal. The Windows and macOS artifacts are therefore not
supported public applications yet, even if their bundles build successfully.
They are useful for detecting platform build issues while the Windows Graphics
Capture and macOS ScreenCaptureKit adapters are implemented.

Before publishing any non-Linux desktop installer, complete the corresponding
native capture support, test the resulting application on that platform, and
add the appropriate Windows code-signing and macOS signing/notarization
release process. Until then, treat downloaded workflow artifacts as internal
test builds.
