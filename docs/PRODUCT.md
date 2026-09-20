# Product definition

## Vision

CaptureVault turns screenshots from disposable files into a dependable personal reference library while keeping the entire workflow on the user's device.

## Primary user

Someone who regularly captures visual information for research, design, troubleshooting, learning, or documentation and wants to retrieve it later without sorting a folder manually.

## Core jobs

1. Capture a specific area without interrupting the current task.
2. Capture a complete display when context matters.
3. Confirm that the capture was saved.
4. Find a previous screenshot using its date, note, tag, or visual thumbnail.
5. Copy or drag a saved screenshot directly into another app or folder.
6. Remove screenshots that are no longer needed.

## Version 0.1 scope

- Ubuntu-first desktop application
- Area and full-screen buttons
- System-owned Wayland permission and selection flow
- Local PNG storage with a SQLite index
- Gallery, search, notes, tags, favorites, copy, native drag-out, and deletion
- Responsive interface suitable for laptop and desktop windows

## Explicit non-goals for 0.1

- Cloud accounts or synchronization
- Sharing links
- Video or GIF capture
- OCR
- Image annotation
- Browser extensions
- Automatic capture based on activity

## Product principles

- **Private by default:** no capture leaves the device without an explicit export action.
- **Fast to trust:** saving is atomic and the capture appears in the library immediately.
- **Respect the desktop:** use compositor and operating-system permission surfaces.
- **Portable core:** platform-specific capture code stays behind a narrow adapter.
- **Quiet interface:** the capture library supports the task instead of becoming another inbox.

## MVP acceptance criteria

- On a supported Ubuntu Wayland session, both capture buttons complete through the XDG portal.
- A successful result is copied into application-managed storage and indexed in SQLite.
- Relaunching the application restores the library.
- Notes, tags, and favorite state persist.
- Search filters the local library without network access.
- Copy publishes both image data and the saved PNG file for inline pasting and file attachments.
- Dragging a capture starts a native copy operation that can drop the PNG into another app or folder.
- Deletion removes both the image and database row after confirmation.
- A cancelled portal request does not create a record or leave the app hidden.
