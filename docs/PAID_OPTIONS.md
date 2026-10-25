# Optional paid service implementation

Version 0.3.4 adds desktop hosted-vault controls. Version 0.3.3 and earlier cannot accept device keys. The v0.3.4 Linux DEB, RPM, AppImage and Apple Silicon DMG are published; all four downloaded installers match their published SHA-256 checksums and GitHub asset digests. Both native release workflows passed.

Setup: open Settings → Optional hosted vault, paste the Library ID and Device key from the website account, and select Connect hosted library. The connection is checked before backup is enabled. Select one local screenshot to back up, or restore a hosted screenshot as a new local copy. Leaving Settings clears the key; reopen Settings and paste it again to reconnect.

The free local MIT desktop app remains functional without an account or payment. The separate `website/` Sites repository implements Personal private hosted backup (CAD $8/month, 1 GB) and Team shared libraries (CAD $15/seat/month, minimum two seats, 5 GB pooled per seat). Native desktop backup sends only a chosen capture and its existing OCR/notes/tags. Restore downloads a private PNG, verifies size and SHA-256, and creates a new local capture with retained metadata. Device keys are held in memory while Settings remains open.

Payments use server-created Stripe subscription Checkout sessions, raw signature-checked subscription webhooks and the customer billing portal. All price IDs and credentials are runtime configuration. CaptureRecall-specific live products, prices, webhook and customer portal are configured in the existing Astra Web Dev Stripe account (acct_1SRKVZAljUfffYWW). Live purchases were enabled in Sites environment revision 3 on October 11, 2026, using a restricted server key. Stripe reports charges enabled and tax settings active. Live unpaid Checkout creation passed for Personal (CAD $8) and Team (two seats, CAD $30); both verification sessions were expired without payment. Live portal creation and production webhook signature validation passed. Earlier real Stripe test subscriptions passed Personal/Team activation, portal creation and cancellation through the service handler. No live charge was performed for verification. Cursay's unused sandbox CaptureRecall webhook remains disabled. Detailed activation instructions, server tests, and known operational limits are in the website repository's README.

The service supports explicit backup/access across devices, rather than automatic synchronization or conflict merging. Browser uploads do not generate OCR. Team invitations grant access to the specified ChatGPT sign-in email; no email is sent. Every route checks current ownership/membership and paid seats. Owners retain export access after expiry, while uploads and shared access stop.

Desktop verification requires frontend check/build and native fmt/tests on Linux and Mac. Hosted verification requires service tests, type checking, Sites build, preview and successful production deployment. The service tests include signed-event isolation from other products on the shared Stripe account. End-to-end signed-in hosted Checkout and actual Stripe subscription webhook delivery remain unverified. Browser QA reached ChatGPT sign-in but the platform returned HTTP 400 invalid content type and invalid_state errors on both the custom domain and generated Site origin; this is an unresolved sign-in issue. Local HMAC delivery and real subscription API tests do not establish those paths. No account-wide merchant settings or other apps' billing resources were modified.

Release verification on October 12, 2026: both platforms passed frontend and native checks, with 23 native tests passing and one sample OCR integration test ignored on each platform. All 14 hosted service tests and website type checks passed. Browser preview confirmed the desktop connection gate and key clearing on leaving Settings. A production request made with the desktop HTTP client reached the live device API and returned the expected JSON 401 for a fabricated key. This does not establish a successful signed-in production backup/restore using a valid account key.

## Valid-key live verification — October 12, 2026

Live testing with the existing owner device key exposed v0.3.4 failing PNG restore because the downloaded temporary file had no extension. v0.3.5 retains .png before import and adds a regression check. Both machines passed native connection, upload, restore, and cross-device restore in both directions, preserving PNG bytes, notes, descriptions, tags, favorite state, OCR metadata and timestamps. New local IDs preserve the original captures. Only synthetic test PNGs were uploaded. All 24 native tests passed on each platform; all 14 hosted service tests passed. Both v0.3.5 release workflows passed and all installer hashes were verified.

Version 0.3.5 is installed on Linux from the verified DEB payload in the user installation directory, and on Mac as /Applications/CaptureRecall.app. The Linux AppImage hit a graphics failure on this NVIDIA system; the native DEB payload runs with per-app graphics compatibility settings. Device credentials were saved outside Git in owner-only files at ~/.config/CaptureRecall/hosted-connection.json on Linux and ~/Library/Application Support/CaptureRecall/hosted-connection.json on Mac, as explicitly requested. The app itself still clears its in-memory key on leaving Settings.

These checks call the desktop native implementation directly. GUI button automation was unavailable; it is not claimed as verified. The in-app browser still hit an OpenAI sign-in error; the existing signed-in Chrome session provided the existing device key without creating or rotating credentials.

## Developer screenshot API — October 25, 2026

The existing Personal and Team subscriptions now include website screenshot API
capacity: 1,000 attempts per UTC calendar month for Personal, and 1,000 per paid
Team seat pooled across the account's API keys. Free accounts receive 25 attempts
subject to a shared free capacity budget. These additions do not change existing
subscription prices or create another billing product. No automatic overage
charges or rollover are enabled. Key management and usage are at
https://capturerecall.com/developers/dashboard/; examples and OpenAPI are linked
from https://capturerecall.com/developers/.

Capture runs in Cloudflare-hosted browsers. Direct PNG/JPEG return is supported;
saving a PNG uses the existing private library authorization and storage quota.
Keys are hashed, revocable, owner/library scoped, and expire after one year.
Atomic D1 reservations enforce quota, rate, concurrency and shared browser-time
budgets before capture. One-use, request-bound tickets authorize the renderer.
Permanent owner hosted Pro remains unlimited for its existing library features,
but developer capture has a bounded 1,000-attempt allowance and the unfunded
global browser budget. Source and operations are in the separate website repo.
