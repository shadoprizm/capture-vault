# Native capture adapters

CaptureVault now routes capture through a small Rust `CaptureProvider` trait.
The provider owns the platform API call and returns a PNG for the shared
library/import path. It does not own storage, metadata, OCR, or enrichment.

| Platform | Full-display capture                                                                   | Selected-area capture  | Minimum runtime                                |
| -------- | -------------------------------------------------------------------------------------- | ---------------------- | ---------------------------------------------- |
| Linux    | XDG Desktop Portal                                                                     | XDG Desktop Portal     | Portal support from the active desktop session |
| macOS    | ScreenCaptureKit `SCScreenshotManager`, first display returned by `SCShareableContent` | Explicitly unavailable | macOS 14.0                                     |
| Windows  | Windows Graphics Capture, primary monitor                                              | Explicitly unavailable | Windows 10 1903 (build 18362) or newer         |

## Honest selection behavior

`Screen` on macOS and Windows is a genuine one-frame capture of a complete
display. It is not a stitched virtual desktop and it does not expose a display
picker yet. The macOS adapter captures the first display ScreenCaptureKit
reports; the Windows adapter captures the primary monitor.

`Area` is deliberately rejected on both platforms with an actionable error.
Neither one-frame API gives CaptureVault an arbitrary rectangular selection
surface. Returning or cropping a whole-display image would silently capture the
wrong thing, so this remains disabled until an app-owned region-selection
overlay can provide coordinates and deal correctly with multiple displays and
mixed DPI. On macOS, a future content-source picker can also offer
Apple's system display/window/app selection UI, but that is distinct from an
arbitrary rectangular region.

Native providers write into a private temporary file held by `CapturedImage`.
The file is removed automatically after the shared store imports it, including
when import fails. Linux portal output remains portal-owned and is never
deleted by CaptureVault.

## macOS setup and release requirements

- The app bundle declares macOS 14.0 as its minimum version because it uses
  `SCScreenshotManager`.
- [`NSScreenCaptureUsageDescription`](../src-tauri/Info.plist) is merged into
  the app bundle so macOS can explain the screen-recording request. On first
  capture, the person must grant **Screen Recording** permission and may need
  to restart the app after granting it.
- Build on a Mac with Xcode and a macOS 14-or-newer SDK. `screencapturekit`
  compiles a small Swift bridge, so macOS packaging cannot be validated on a
  Linux-only runner.
- Before public distribution, sign the `.app` with a stable Developer ID or
  Apple Distribution identity, enable the hardened runtime, and notarize the
  DMG/app. The stable bundle identifier matters because macOS associates the
  Screen Recording permission with the app identity.
- This one-shot foreground capture does not configure a background capture
  mode. If the product later captures while backgrounded or enters the App
  Sandbox, revisit Apple's current Signing & Capabilities requirements rather
  than copying unrelated entitlements into this app.

Apple's [ScreenCaptureKit overview](https://developer.apple.com/documentation/screencapturekit)
requires a screen-capture usage description and user permission; its
[macOS capture sample](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos)
documents the restart after first-time permission.

## Windows setup and release requirements

- Build on Windows with the MSVC Rust target, Visual Studio Build Tools, and a
  Windows SDK that includes Windows Graphics Capture. The adapter uses the
  Win32 `IGraphicsCaptureItemInterop::CreateForMonitor` path through the
  maintained `windows-capture` crate.
- Windows Graphics Capture is checked at runtime by the crate. The monitor
  interop route requires Windows 10 version 1903 (build 18362) or later; the
  adapter returns a native capture error if Windows reports that capture is not
  supported.
- This is a Tauri desktop/Win32 bundle, not a UWP app. If the release format is
  changed to MSIX/UWP, review and add the applicable Graphics Capture package
  capability before shipping.
- Authenticode signing and timestamping are distribution requirements for a
  trustworthy installer and better SmartScreen reputation, not prerequisites
  for the capture API itself. Test signed release artifacts on actual Windows
  hardware, including HDR and multi-monitor setups.

Microsoft's [screen-capture guide](https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture)
documents the `GraphicsCaptureSession` support check and one-frame capture
pipeline. Its [desktop interop reference](https://learn.microsoft.com/en-us/windows/win32/api/windows.graphics.capture.interop/nn-windows-graphics-capture-interop-igraphicscaptureiteminterop)
lists Windows 10 version 1903 as the minimum for `CreateForMonitor`.

## Verification still required

Linux tests only prove that the platform boundary does not regress the current
portal implementation. Before advertising either new desktop target, manually
verify a signed release build on the native OS:

1. Grant or deny the permission and confirm the error remains useful.
2. Capture the selected full display at normal and mixed-DPI scaling.
3. Verify the Tauri window is not included after its normal hide step.
4. Verify the PNG imports, temporary capture output is removed, and OCR can
   open the stored image.
5. Exercise a protected/HDR surface and confirm the product behavior is
   documented instead of claiming a successful capture.
