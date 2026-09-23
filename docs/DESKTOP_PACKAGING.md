# Desktop packaging

`package-desktop.yml` is the manual integration build for Ubuntu, Windows, and
macOS. It keeps generated bundles as GitHub Actions artifacts for 14 days and
does not publish a release.

`release-linux.yml` publishes tagged Linux prereleases. `release-macos.yml`
builds an Apple Silicon DMG on a native `macos-26` runner, validates its code
signature, generates a SHA-256 checksum, and uploads both files to the matching
GitHub prerelease.

## macOS signing modes

The Mac release workflow supports two modes:

- With `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
  `KEYCHAIN_PASSWORD`, `APPLE_SIGNING_IDENTITY`, and Apple notarization
  credentials configured as repository secrets, Tauri signs, notarizes, and
  staples the direct-download build.
- Without those secrets, the workflow uses ad-hoc signing and labels the output
  as an internal preview. Ad-hoc output is installable for testing but is not a
  Gatekeeper-approved public release.

For notarization, configure either App Store Connect API credentials
(`APPLE_API_ISSUER`, `APPLE_API_KEY`, and the `.p8` key) or the Apple ID flow
documented by Tauri. The certificate must be **Developer ID Application**;
Apple Development and Apple Distribution certificates cannot notarize a
direct-download DMG.

The app keeps hardened runtime enabled and its long-lived bundle identifier.
Do not change the identifier solely for the CaptureRecall rename because that
would create a new macOS permission identity and application-data directory.
