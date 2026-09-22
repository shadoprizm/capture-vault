# Architecture

## Overview

CaptureVault is a Tauri application with a React interface and a Rust core. Capture APIs are platform-specific; storage and product behavior are shared.

```text
React interface
  ├─ capture controls
  ├─ library, OCR search, copy, and native drag-out
  └─ title, description, note, and tag editor
          │ Tauri commands
Rust core
  ├─ capture adapter
  │    ├─ Linux: XDG Desktop Portal
  │    ├─ macOS: ScreenCaptureKit full-display still capture
  │    └─ Windows: Windows Graphics Capture primary-monitor still capture
  ├─ local enrichment engine
  │    ├─ bundled OCR detection and recognition models
  │    └─ loopback-only semantic vision client
  └─ capture store
       ├─ application-managed PNG files
       └─ versioned SQLite metadata index
```

## Capture flow

1. A capture starts from an interface button or a registered global shortcut. Shortcut choices are validated, registered through Tauri, and persisted in local application storage.
2. The React window hides so it is not included in the screenshot. A shortcut-triggered capture preserves the window's previous visibility instead of bringing a hidden window forward.
3. The frontend invokes `capture_screen` with `area` or `screen`.
4. The active platform provider requests a screenshot: Linux uses `org.freedesktop.portal.Screenshot`; macOS uses ScreenCaptureKit; and Windows uses Windows Graphics Capture.
5. Portal version 3 receives an explicit target. Older Linux portals fall back to the interactive behavior. macOS and Windows currently support only the full-display path and return an explicit error for area selection.
6. The returned or provider-created PNG is validated and imported into CaptureVault storage.
7. Provider-created temporary PNGs are removed after import; portal output remains portal-owned.
8. The file is copied to a temporary name and atomically renamed.
9. Metadata is committed to SQLite.
10. The new record is returned to the gallery and the window's previous visibility is restored.
11. A background task runs local OCR and, only after explicit opt-in, whole-image semantic vision analysis, then emits the updated record to the interface.

## Enrichment flow

- The `ocrs` engine memory-maps its immutable RTen detection and recognition models from bundled application resources once at startup.
- OCR always runs on a blocking worker. Vision inference runs alongside OCR only after `CAPTURE_VAULT_ENABLE_VISION=1` explicitly opts in; OCR access is serialized so rapid captures do not compete for CPU and memory.
- Opted-in semantic metadata comes from the local OpenAI-compatible multimodal service at `127.0.0.1:8083` using `Gemma 4 26B-A4B - Fast General` by default. Environment variables can select another loopback endpoint and model; non-loopback URLs are rejected and HTTP redirects are never followed.
- The opted-in vision model receives the complete image and a strict JSON schema. Its prompt asks for the screen's application, activity, and purpose—not copied OCR—and treats all text inside the image as untrusted content rather than instructions. CaptureVault cannot control whether a separately run local service relays that image after receiving it.
- Raw detected text is stored separately from the title, description, and personal note so all visible text is searchable.
- The database tracks whether a title or description has been edited. Re-analysis can replace an earlier generated suggestion but preserves user edits and never touches notes.
- Records expose `pending`, `processing`, `complete`, `partial`, or `failed` status so the interface can distinguish searchable OCR from completed semantic analysis and offer a retry.
- The bundled recognition model targets Latin-alphabet text. Its source, pinned revision, license, and checksums are recorded in `src-tauri/resources/ocr/NOTICE.md`.

## Storage

The database stores relative file paths, UTC timestamps, image dimensions, capture type, title, description, edit provenance, note, serialized tags, favorite state, OCR text, and enrichment status. Schema version 3 migrates older libraries in place. The Tauri command returns an absolute path only for local display through the scoped asset protocol.

Images and metadata are deliberately separate:

- SQLite makes filtering and future migrations predictable.
- Plain PNG files remain inspectable and recoverable.
- Relative database paths allow a future library relocation feature.

## Reuse flow

- Copy resolves a capture by its library ID, then publishes both `image/png` and the PNG file list to the system clipboard. This lets rich editors paste the pixels while email clients, upload controls, and file managers can consume the original file.
- Linux additionally publishes the GNOME copy-files MIME type so pasting into a folder is treated as a copy, not a move.
- Dragging a library card or the large preview hands the absolute PNG path to the operating system's native drag session in copy mode. The destination receives a real file rather than a web image URL.
- Transfer actions never duplicate or relocate the library's managed source file.

## Failure behavior

- A failed file copy does not insert a database row.
- A failed database insert removes the newly copied image.
- A failed enrichment task leaves the screenshot intact, records a retryable status, and never modifies user-authored metadata.
- Deletion first stages the image, restores it if the database operation fails, and removes it after the row is deleted.
- Cancelling the desktop portal returns to the application without creating a capture.

## Security boundaries

- The asset protocol is enabled only for the application-data directory.
- The content security policy allows application assets and the Tauri local asset protocol only.
- No remote image sources, telemetry, OCR API, or upload service are configured. OCR is local. Semantic vision is disabled by default; after explicit opt-in it is hard-limited to a loopback HTTP endpoint and refuses redirects, but the separately run local service is responsible for its own handling of received images.
- The XDG portal remains responsible for Wayland capture permissions and selection UI.

## Next architectural steps

- Add a native, multi-display-aware area-selection overlay before enabling `area` on macOS or Windows. Do not crop a full-display capture as a substitute for selection.
- Generate cached thumbnails rather than loading original images in large libraries.
- Add a SQLite full-text index when library scale justifies it.
- Add a trash/recovery policy before replacing permanent deletion.
