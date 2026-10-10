# Optional paid service implementation

Implemented on `codex/reconcile-linux-work`; do not describe its desktop controls as present in the published v0.3.3 packages.

The free local MIT desktop app remains functional without an account or payment. The separate `website/` Sites repository implements Personal private hosted backup (CAD $8/month, 1 GB) and Team shared libraries (CAD $15/seat/month, minimum two seats, 5 GB pooled per seat). Native desktop backup sends only a chosen capture and its existing OCR/notes/tags. Restore downloads a private PNG, verifies size and SHA-256, and creates a new local capture with retained metadata. Device keys are held in memory while Settings remains open.

Payments use server-created Stripe subscription Checkout sessions, raw signature-checked subscription webhooks and the customer billing portal. All price IDs and credentials are runtime configuration. Checkout is disabled until the actual merchant account is configured; no real payment has been tested or accepted. Detailed activation instructions, server tests, and known operational limits are in the website repository's README.

The service supports explicit backup/access across devices, rather than automatic synchronization or conflict merging. Browser uploads do not generate OCR. Team invitations grant access to the specified ChatGPT sign-in email; no email is sent. Every route checks current ownership/membership and paid seats. Owners retain export access after expiry, while uploads and shared access stop.

Desktop verification requires frontend check/build and native fmt/tests on Linux and Mac. Hosted verification requires service tests, type checking, Sites build, preview and successful production deployment. A real Stripe test purchase is still required before live billing activation; simulated provider responses prove code paths only.
