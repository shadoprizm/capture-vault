# Architecture

## Overview

CaptureVault is a Tauri application with a React interface and a Rust core. Capture APIs are platform-specific; storage and product behavior are shared.

```text
React interface
  ├─ capture controls
  ├─ library and search
  └─ details editor
          │ Tauri commands
Rust core
  ├─ capture adapter
  │    ├─ Linux: XDG Desktop Portal
  │    ├─ macOS: ScreenCaptureKit (planned)
  │    └─ Windows: Windows Graphics Capture (planned)
  └─ capture store
       ├─ application-managed PNG files
       └─ SQLite metadata index
```

## Capture flow

1. The React window hides so it is not included in the screenshot.
2. The frontend invokes `capture_screen` with `area` or `screen`.
3. Linux requests a screenshot from `org.freedesktop.portal.Screenshot`.
4. Portal version 3 receives an explicit target. Older versions fall back to the interactive behavior.
5. The returned file URI is validated and imported into CaptureVault storage.
6. The file is copied to a temporary name and atomically renamed.
7. Metadata is committed to SQLite.
8. The new record is returned to the gallery and the window is restored.

## Storage

The database stores relative file paths, UTC timestamps, image dimensions, capture type, note, serialized tags, and favorite state. The Tauri command returns an absolute path only for local display through the scoped asset protocol.

Images and metadata are deliberately separate:

- SQLite makes filtering and future migrations predictable.
- Plain PNG files remain inspectable and recoverable.
- Relative database paths allow a future library relocation feature.

## Failure behavior

- A failed file copy does not insert a database row.
- A failed database insert removes the newly copied image.
- Deletion first stages the image, restores it if the database operation fails, and removes it after the row is deleted.
- Cancelling the desktop portal returns to the application without creating a capture.

## Security boundaries

- The asset protocol is enabled only for the application-data directory.
- The content security policy allows application assets and the Tauri local asset protocol only.
- No remote image sources, telemetry, or upload service are configured.
- The XDG portal remains responsible for Wayland capture permissions and selection UI.

## Next architectural steps

- Introduce a formal `CaptureProvider` trait before adding the second platform adapter.
- Move schema changes to numbered migrations before version 0.2.
- Generate cached thumbnails rather than loading original images in large libraries.
- Add full-text search when OCR or larger note collections justify it.
- Add a trash/recovery policy before replacing permanent deletion.
