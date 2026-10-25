# CaptureRecall source of truth

GitHub `main` in https://github.com/shadoprizm/capture-vault is the shared desktop baseline. Hosted desktop setup was merged through PR #5 on 2026-10-12 at `264196e8d2b0f22498033476bff4c0a745118af9`. The v0.3.4 code tag points to `4bf1d0daac2c6f38e5b01b45e683eff52cb8db9b`; all six preview assets are published and all four downloaded installers match both GitHub asset digests and their checksum files. Reconciliation was merged through PR #3 on 2026-10-10 at `780b5a51ac2768b20e94992a1dc3d86476ea3a5c`, which is the code commit tagged `v0.3.3`. All six preview release assets are published. Later documentation commits may advance main without changing that release code.

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

## Hosted desktop setup release — October 12, 2026

Version 0.3.4 adds Settings → Optional hosted vault with Library ID, Device key, and Connect hosted library. The app checks access before enabling selected-screenshot backup and supports integrity-checked restore. The key clears when leaving Settings. The free local app still works without an account. Version 0.3.3 and earlier cannot accept a device key.

Website version 11, source `b769411668a2ca320b32f8999d87ce40ab8d4742`, deployed successfully to `https://capturerecall.shadoprizm.chatgpt.site` and is served at capturerecall.com. Account setup instructions, copy buttons, download links, installer sizes and digests now match v0.3.4. Selecting another library clears the previous displayed key. Both release workflows, PR #5 CI, and merged-main CI passed on Linux and Mac. All four installer downloads match GitHub digests and checksum files; the DEB reports version 0.3.4 and amd64. Both packages built the same frontend asset verified in the browser preview. Mac signature verification passed with an ad-hoc signature; notarization remains pending.

The production device API returned the expected JSON 401 to a fabricated key sent with the desktop HTTP client. Valid-key production backup/restore and physical capture still require native account/device validation. No customer screenshots were used for verification.

## Live verification and restore fix — October 12, 2026

v0.3.5, code tag `1f958df0dd3dd65ff2dd1b7fea22b6c3b977b262`, was merged through PR #7 at `b69f812c3345cd0ee621ad63bb502611c76c1c54`. A valid-key live test found v0.3.4 could upload but failed to restore an extensionless temporary PNG. v0.3.5 adds .png before import and a regression test. Both platforms passed 24 native tests and live connection, upload, restore and cross-device restore using only synthetic images. Pixels and metadata match; restored captures receive new local IDs. All four downloaded installers match checksum files and GitHub digests; both release workflows passed. v0.3.5 is installed on both user machines. Linux uses the native DEB payload in ~/.local/opt/CaptureRecall/0.3.5 through ~/.local/bin/capture-vault; the AppImage hit EGL/graphics initialization failures on this NVIDIA desktop. Mac uses /Applications/CaptureRecall.app, with its signature and version verified. Earlier app copies and database snapshots were preserved.

Website version 12, source `6d9f742ecdd09478f0053fcb9b68b39d0b5bcdf6`, advertises verified v0.3.5 packages and warns that v0.3.4 restore is broken. Credentials remain outside Git in owner-only private files. Live checks used the native desktop implementation; GUI button interaction and physical screen capture were not automated.

## Developer API publication — October 25, 2026

Website version 14, source `3c62e3d805f207143bfe4f43abf82c50dc9db2bc`, deployed
successfully at https://capturerecall.com using environment revision 5. The Mac
website checkout is `/Users/jratelle/Coding Projects/capture-vault/website`, a
separate Sites repository with the same project identity as the Linux checkout.
The Linux website checkout was clean at `6d9f742` before publication; it needs a
fresh authenticated source fetch before its next edit. Do not overwrite it from
a folder copy. Desktop source and packages remain v0.3.5.

The publication adds developer keys, website capture, usage and hard quotas,
private PNG library saving, OpenAPI documentation, updated product/pricing/privacy
copy, structured data and crawler facts. Twenty-six service/marketing tests,
TypeScript checking and the production build passed. Six shared renderer tests
and an actual cross-service handler/ticket contract test also passed. Local D1
HTTP tests verified key creation/authentication/revocation and ticket replay
rejection. The production runtime dependency audit reports zero advisories after
compatible Next, Sharp and supporting dependency updates.

Public page/API rejection checks and the signed-in owner dashboard passed after
publication. The first two authorized live attempts exposed Workers rejecting
the standard Fetch `redirect: error` mode before sending the renderer request.
The temporary key was revoked, both attempts counted, and both outbound hops now
use manual redirects with non-OK rejection. A Workers runtime reproduction and
redirect regression checks passed. The renderer correction merged in VibersVault
PR #32 and deployed as version `c3037a3a-a52d-4b47-b50e-5301eea836d0`.
The owner-approved replacement-key production test then passed twice. Each public
homepage capture returned HTTP 200 with a fully loaded 1280 × 720 PNG; account
usage rose from two to four attempts. The temporary key was immediately revoked
and the dashboard confirmed no active developer keys. Images were returned only,
not saved to the hosted library, and the credential stayed in page memory. Both
Linux and macOS CI jobs for the documentation launch PR #9 passed.
Native desktop release verification remains in the earlier v0.3.5 record; this
release does not ship a new desktop installer.
