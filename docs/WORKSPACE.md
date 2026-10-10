# CaptureRecall source of truth

GitHub `main` in https://github.com/shadoprizm/capture-vault is the shared desktop baseline. On 2026-10-10 its verified commit was `ad298faf5015ec65847bb809009c50bd65c9f072`; the newest published preview was `v0.3.2`.

## Active checkouts

| Purpose | Location | Working branch / source |
| --- | --- | --- |
| Linux desktop | `/home/shadoprizm/Documents/ChatGPT/Capture Vault - Screenshot Archives and Tracking` | `codex/reconcile-linux-work` |
| Mac desktop | `/Users/jratelle/Coding Projects/capture-vault` | `codex/reconcile-linux-work` |
| Website | `website/` inside the Linux project | Independent Sites repository, `main` |

The reconciliation branch starts at current desktop main. It retains Wayland portal shortcuts, reports OCR-only and vision-only outcomes accurately, surfaces analysis warnings, accepts IPv6 loopback services, and saves vision choices atomically. Native Mac shortcut handling, permission fixes, capture window lifecycle, and editable endpoint/model settings remain from current main. Legacy Linux `analysis-settings.json` is read only when the current `vision-settings.json` is absent; the next save uses the current format. These additions are development work, not published release claims.

## Website authority

The website is https://capturerecall.com, owned by Sites project `appgprj_6ab44eb46e9481918d250f5f9479e7ce`. Its independent Git checkout is `website/`; its `.openai/hosting.json` declares that same project. The latest deployed version verified during consolidation was version 3, source commit `b04d2561c23928fde7e59107abd0b8c68372110b`, matching the clean local website checkout. It has no ordinary `origin` remote; use the Sites source workflow and fresh scoped credentials for future synchronization. Do not publish from a copied directory or assume desktop Git includes the website. Root Git ignores this nested repository.

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

Run frontend type checks and build, Rust format checks and tests on Linux, and Rust tests on the Mac for cross-platform changes. Portal permission dialogs and physical capture still require a supported native desktop session; unit tests do not establish those behaviors. Website publishing and desktop releases are separate operations.
