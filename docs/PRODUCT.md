# Product definition

## Vision

CaptureVault turns screenshots from disposable files into a dependable personal reference library while keeping the default workflow on the user's device.

## Primary user

Someone who regularly captures visual information for research, design, troubleshooting, learning, or documentation and wants to retrieve it later without sorting a folder manually.

## Core jobs

1. Capture a specific area without interrupting the current task.
2. Capture a complete display when context matters.
3. Confirm that the capture was saved.
4. Find a previous screenshot using its title, description, visible text, note, tag, date, or visual thumbnail.
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

## Version 0.2 scope

- Configurable global shortcuts for area and full-screen capture
- Automatic, on-device OCR after every new capture
- Explicitly opt-in semantic title and description suggestions based on the whole image
- Full detected text in capture details and library search
- Re-analysis that preserves personal notes and manual edits
- In-place migration of existing version 0.1 libraries

## Explicit non-goals for 0.2

- Uploading captures to a hosted vision model
- Guaranteed recognition outside the Latin alphabet
- Automatic tags or actions based on screenshot content

## Product principles

- **Private by default:** no capture leaves CaptureVault without an explicit export action or explicit opt-in to a vision service. A loopback endpoint may forward to another computer through a user-configured tunnel.
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
- Newly captured screenshots receive local OCR without blocking capture or transfer; optional vision analysis runs only after explicit opt-in.
- Re-running enrichment does not overwrite personal notes or edited titles and descriptions.
- Copy publishes both image data and the saved PNG file for inline pasting and file attachments.
- Dragging a capture starts a native copy operation that can drop the PNG into another app or folder.
- Deletion removes both the image and database row after confirmation.
- A cancelled portal request does not create a record or leave the app hidden.
