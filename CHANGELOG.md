# Changelog

All notable changes to CaptureVault are documented here.

## 0.2.0 — 2026-09-21

This is a Linux prerelease. Windows and macOS capture adapters are experimental
source support only; no public installers are published for either platform.

### Added

- Persistent configurable global shortcuts for area and full-display capture.
- Local OCR with bundled RTen models, searchable detected text, and retryable
  enrichment status.
- Editable title and description suggestions, storage-location migration, and
  richer capture metadata.
- A manual native-runner packaging workflow for Linux, Windows, and macOS.
- A strict-confinement Snap package foundation and release guidance.
- Experimental full-display providers for macOS ScreenCaptureKit and Windows
  Graphics Capture; selected-area capture remains intentionally unavailable on
  those platforms.

### Privacy and reliability

- Optional whole-image vision analysis is now off by default and requires
  `CAPTURE_VAULT_ENABLE_VISION=1`.
- Vision requests refuse redirects so a loopback endpoint cannot forward the
  screenshot body through CaptureVault.
- Storage migration is serialized with imports and persists its settings via a
  temporary file replacement, preventing shortcut captures from being lost
  during a folder move.

### Distribution

- Linux `.deb`, RPM, and AppImage bundles include the OCR models and their
  CC-BY-SA-4.0 attribution notice.
- GitHub release automation creates a Linux prerelease with SHA-256 checksums
  when a matching version tag is pushed.

## 0.1.2 — 2026-09-20

- Linux screenshot transfer release with `.deb`, RPM, and AppImage downloads.
