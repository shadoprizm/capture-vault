fn main() {
    // screencapturekit's Swift bridge links libswift_Concurrency via @rpath.
    // macOS ships that runtime in /usr/lib/swift, but test binaries do not get
    // an LC_RPATH automatically. Keep native tests and app bundles executable.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
    tauri_build::build()
}
