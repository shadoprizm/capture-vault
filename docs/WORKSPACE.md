# CaptureRecall source of truth

GitHub `main` in https://github.com/shadoprizm/capture-vault is the shared desktop baseline. Reconciliation was merged through PR #3 on 2026-10-10 at `780b5a51ac2768b20e94992a1dc3d86476ea3a5c`, which is the code commit tagged `v0.3.3`. All six preview release assets are published. Later documentation commits may advance main without changing that release code.

## Active checkouts

| Purpose | Location | Working branch / source |
| --- | --- | --- |
| Linux desktop | `/home/shadoprizm/Documents/ChatGPT/Capture Vault - Screenshot Archives and Tracking` | `codex/reconcile-linux-work` |
| Mac desktop | `/Users/jratelle/Coding Projects/capture-vault` | `codex/reconcile-linux-work` |
| Website | `website/` inside the Linux project | Independent Sites repository, `main` |

The reconciliation branch starts at current desktop main. It retains Wayland portal shortcuts, reports OCR-only and vision-only outcomes accurately, surfaces analysis warnings, accepts IPv6 loopback services, and saves vision choices atomically. Native Mac shortcut handling, permission fixes, capture window lifecycle, and editable endpoint/model settings remain from current main. Legacy Linux `analysis-settings.json` is read only when the current `vision-settings.json` is absent; the next save uses the current format. These changes shipped in the v0.3.3 preview. Portal behavior still depends on the native desktop; Mac notarization remains pending.

## Website authority

The website is https://capturerecall.com, owned by Sites project `appgprj_6ab44eb46e9481918d250f5f9479e7ce`. Its independent Git checkout is `website/`; its `.openai/hosting.json` declares that same project. The latest deployed version verified during consolidation was version 3, source commit `b04d2561c23928fde7e59107abd0b8c68372110b`, matching the clean local website checkout. Marketing publication on 2026-10-10 advanced the website to version 4 at source commit `cf0ff936f17946a786b21a3ebbf12731293c62db`. Production release follow-up published version 5 at source commit `17d2f29fe2918da956d59423e7dbc9597fef9219`, advertising verified v0.3.3 packages and checksums. The Sites helper configures its source remote; use the Sites source workflow and fresh scoped credentials for future synchronization. Do not publish from a copied directory or assume desktop Git includes the website. Root Git ignores this nested repository.

## Synchronization

Before starting work, check status and branch, fetch origin, and compare against current remote main. Use the existing checkout and the selected task branch. Keep uncommitted work backed up and reconcile it deliberately. Transfer shared work through Git commits and branches, not folder copies. Keep one active desktop checkout per machine. Merge branch work through the normal review process; do not force-push main.

## Preserved archives

The 2026-10-10 consolidation archive is `/home/shadoprizm/Documents/Project Archives/CaptureRecall/20261010-080827`. It contains:

- `linux-history.bundle`: verified complete pre-consolidation Git history.
- `linux-working-files.tar.gz`: tracked and untracked working files, including the separate website repository. Build caches and generated desktop build output are excluded.
- `original-status.txt`: the original dirty working state.
- `obsolete-linux-prototype/`: the entire former prototype checkout, including its ignored files and build artifacts. Its clean HEAD `1c8ec21` is an ancestor of current main.
- `obsolete-site-temporary-checkout/`: the redundant clean temporary website checkout at the same deployed commit.
- `research-cache/`: preserved local Firecrawl research files.

The original development edits also remain in Git stash `CaptureRecall pre-consolidation OCR and settings work`; keep it until the reconciliation branch has been reviewed. Restore historical files into a temporary recovery directory, then compare them with the active checkout before applying changes.

## Validation

On the Mac, use the installed full Xcode and the supported deployment target in the validation shell, without changing machine-wide settings:

```bash
export DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo test --manifest-path src-tauri/Cargo.toml
```

The default Command Line Tools selection and implicit macOS 11 linker target failed Swift compatibility-library linking during consolidation. Full Xcode and an explicit macOS 14 target are required for this native check.

Run frontend type checks and build, Rust format checks and tests on Linux, and Rust tests on the Mac for cross-platform changes. Portal permission dialogs and physical capture still require a supported native desktop session; unit tests do not establish those behaviors. Website publishing and desktop releases are separate operations.

## Production verification

PR #3 CI and merged-main CI passed on Linux and Mac. Both v0.3.3 release workflows succeeded, including the Mac bundle signature check; the signature is ad-hoc, not Apple-notarized. All four installer digests match their published checksum files, and the downloaded DEB checksum and version/architecture were independently verified. Website version 5 deployment succeeded. Homepage and Linux guide mobile emulation passed at 390px width, including navigation and no document overflow. Physical capture and portal permission behavior still need broader native testing. GitHub Copilot review did not complete because its weekly rate limit was exhausted; this is not a completed independent code review.
