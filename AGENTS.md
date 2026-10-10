# CaptureRecall working repository

- Desktop source: `https://github.com/shadoprizm/capture-vault.git`. The product is CaptureRecall; keep the repository name and application identifier stable.
- Use this existing checkout on Linux and `/Users/jratelle/Coding Projects/capture-vault` on the Mac. Do not create another clone just to continue project work.
- At the start of repository work, inspect `git status`, branch, and HEAD, then fetch `origin` and compare against `origin/main`. Preserve uncommitted work before synchronization; never assume a local tracking ref is current.
- Work in the explicitly selected task branch. Reconciliation work lives on `codex/reconcile-linux-work`; it includes current main plus retained Linux changes. Do not describe branch-only changes as released.
- `website/` is a separate Sites Git repository. Do not add it to desktop Git or deploy it as part of desktop work. Consult `docs/WORKSPACE.md` for its Site identity and source verification.
- Before advertising a feature or package, verify both implementation and published release assets. The latest preview may differ from GitHub's latest non-prerelease endpoint.
- Required checks for changes spanning native and frontend code: `npm run check`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
- On the Mac, run native validation with `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer` and `MACOSX_DEPLOYMENT_TARGET=14.0` set in that shell. Do not change machine-wide Xcode selection.
