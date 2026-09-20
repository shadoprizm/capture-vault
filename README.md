# CaptureVault

CaptureVault is a private, local-first screenshot library. Capture an area or a full display, then keep the result in a searchable visual archive with notes, tags, and favorites.

> **Status:** early Ubuntu-first prototype. Linux capture uses the XDG Desktop Portal. The storage and interface foundations are cross-platform; native macOS and Windows capture adapters are planned.

## What works

- Capture a selected area or full screen through the Linux system portal
- Keep PNG files and metadata locally
- Browse a responsive thumbnail library
- Search filenames, notes, tags, and capture types
- Add notes, tags, and favorites
- Permanently delete a capture and its metadata
- Run without an account or network service

## Technology

- [Tauri 2](https://tauri.app/) desktop shell
- React and TypeScript interface
- Rust application core
- SQLite metadata store
- XDG Desktop Portal integration through [ashpd](https://github.com/bilelmoussaoui/ashpd)

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the system design and [docs/PRODUCT.md](docs/PRODUCT.md) for the product scope.

## Ubuntu development setup

Install the Tauri Linux prerequisites:

```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev build-essential curl file \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libdbus-1-dev pkg-config
```

Install a current stable Rust toolchain and Node.js 22 or newer. Then:

```bash
npm install
npm run tauri dev
```

The browser-only interface preview is available with `npm run dev`, but screen capture and the persistent library require the Tauri desktop runtime.

## Checks

```bash
npm run check
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```

## Privacy

CaptureVault stores screenshots and its SQLite index in the operating system's application-data directory. It does not upload screenshots or include analytics. Screenshots can contain sensitive information; review a capture before sharing it outside the application.

## Roadmap

1. Validate the capture flow across Ubuntu GNOME Wayland, Ubuntu X11, multiple displays, and mixed scaling.
2. Add tray controls and configurable global shortcuts.
3. Add copy, export, reveal-in-folder, and configurable library locations.
4. Implement macOS capture with ScreenCaptureKit.
5. Implement Windows capture with Windows Graphics Capture.
6. Add optional OCR and annotation after the core library is stable.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). By participating, you agree to keep CaptureVault local-first and privacy-respecting.

## License

[MIT](LICENSE)
