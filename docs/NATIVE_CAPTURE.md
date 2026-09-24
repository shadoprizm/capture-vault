# Native capture adapters

CaptureRecall routes capture through a small Rust `CaptureProvider` trait. The
provider owns the platform API call and returns a PNG for the shared
library/import path. It does not own storage, metadata, OCR, or enrichment.

| Platform | Full-display capture | Selected-area capture | Minimum runtime |
| --- | --- | --- | --- |
| Linux | XDG Desktop Portal | XDG Desktop Portal | Portal support from the active desktop session |
| macOS | ScreenCaptureKit `SCScreenshotManager`, main display | Native macOS crosshair picker | macOS 14.0 |
| Windows | Windows Graphics Capture, primary monitor | Explicitly unavailable | Windows 10 1903 (build 18362) or newer |

## macOS behavior

Full-screen capture resolves the CoreGraphics main-display identifier against
the displays returned by ScreenCaptureKit and captures one frame with
`SCScreenshotManager`. Area capture launches macOS' built-in interactive
selection surface in selection-only mode. The app window is hidden before
either provider runs. Press Escape to cancel area selection without creating a
library item. The app checks its effective Screen Recording permission before
launching the picker. If macOS still denies access while the Settings switch
appears on, turn that switch off and on, then fully quit and reopen CaptureRecall.

Native providers write into a private temporary file held by `CapturedImage`.
The file is removed automatically after the shared store imports it, including
when import fails. Linux portal output remains portal-owned and is never
deleted by CaptureRecall.

The bundle keeps the original identifier
`io.github.shadoprizm.capturevault` while the public product name changes to
CaptureRecall. This is deliberate: it preserves the existing application-data
directory and macOS Screen Recording permission identity for upgrades.

## macOS setup and release requirements

- The app bundle declares macOS 14.0 as its minimum version because it uses
  `SCScreenshotManager`.
- [`NSScreenCaptureUsageDescription`](../src-tauri/Info.plist) is merged into
  the app bundle. On first capture, the person must grant **Screen & System
  Audio Recording** permission and may need to restart the app.
- Build on Apple hardware with Xcode and a macOS 14-or-newer SDK.
  `screencapturekit` compiles a Swift bridge. `build.rs` supplies the system
  Swift runtime rpath needed by native test executables.
- Direct-download releases must be signed with a **Developer ID Application**
  certificate, use hardened runtime, be submitted to Apple's notary service,
  and have the notary ticket stapled. An Apple Development or Apple
  Distribution identity is not a substitute for Developer ID distribution.
- Ad-hoc signed artifacts are suitable for internal preview testing only and
  will require manual approval in Privacy & Security when downloaded.

Apple's [ScreenCaptureKit overview](https://developer.apple.com/documentation/screencapturekit)
and [macOS capture sample](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos)
describe the permission and capture model. Apple's
[notarization guide](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
defines the direct-download signing requirements.

## Windows behavior

Full-screen capture uses Windows Graphics Capture for the primary monitor.
Area capture remains rejected with an actionable error because the one-frame
API does not provide an arbitrary region-selection surface. Returning or
cropping an unrequested full display would silently capture the wrong content.

Build on Windows with the MSVC Rust target, Visual Studio Build Tools, and a
Windows SDK that includes Windows Graphics Capture. Authenticode signing and
timestamping remain distribution requirements for a trustworthy installer.

## Native verification checklist

Before promoting a build from preview to supported:

1. Grant and deny screen-recording permission and confirm both paths are clear.
2. Capture full display and a selected area, including cancel with Escape.
3. Verify the CaptureRecall window is absent from the image.
4. Verify import, thumbnail, OCR, search, copy, drag, and deletion.
5. Verify global shortcuts while the main window is hidden and visible.
6. Exercise multiple displays, Retina scaling, and a protected/HDR surface.
7. Validate the final installed bundle with `codesign`, `spctl`, and `stapler`.
