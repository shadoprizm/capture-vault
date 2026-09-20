# Contributing

CaptureVault is at an early stage. Small, focused changes with a clear user outcome are easiest to review.

## Development workflow

1. Create a topic branch from `main`.
2. Keep platform-specific capture behavior inside `src-tauri/src/capture.rs` or a future platform module.
3. Add tests for storage behavior and any pure capture-selection logic.
4. Run the checks documented in the README.
5. Describe the tested desktop session in the pull request, especially for Wayland changes.

## Design constraints

- Do not add analytics or uploads by default.
- Preserve the system portal permission flow on Linux.
- Avoid storing absolute paths in the database.
- Keep destructive actions explicit and confirmed.
- Ensure the app window is restored after capture errors or cancellation.

## Reporting bugs

Include the Ubuntu release, desktop environment, display server (`Wayland` or `X11`), portal backend, monitor layout, and scaling configuration where relevant. Never attach a private screenshot to a public issue without reviewing its contents.
