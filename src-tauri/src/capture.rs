//! Platform capture adapters.
//!
//! Every provider returns a PNG source for the shared capture store. Providers
//! own only the operating-system interaction; importing, indexing, and
//! enrichment remain platform-independent.

use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

use thiserror::Error;

#[cfg(any(target_os = "macos", target_os = "windows", test))]
use tempfile::TempPath;

use crate::models::CaptureMode;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod unsupported;
#[cfg(target_os = "windows")]
mod windows;

/// The asynchronous result returned by a platform capture provider.
pub(crate) type CaptureFuture =
    Pin<Box<dyn Future<Output = Result<CapturedImage, CaptureError>> + Send>>;

/// Narrow boundary between platform screen-capture APIs and the shared app.
///
/// Providers must return a valid PNG path. A provider-created temporary file is
/// held by [`CapturedImage`] and removed after CaptureVault imports it. Paths
/// returned by an operating-system portal remain owned by that portal.
pub(crate) trait CaptureProvider: Send + Sync {
    fn capture(&self, mode: CaptureMode) -> CaptureFuture;
}

/// A screenshot handed from a platform provider to the capture store.
///
/// `temporary_path` deliberately owns the file instead of relying on a
/// best-effort cleanup call. Once the store has copied the image, dropping this
/// value removes native provider output even when the import fails.
#[derive(Debug)]
pub(crate) struct CapturedImage {
    path: PathBuf,
    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    _temporary_path: Option<TempPath>,
}

impl CapturedImage {
    pub(crate) fn from_portal(path: PathBuf) -> Self {
        Self {
            path,
            #[cfg(any(target_os = "macos", target_os = "windows", test))]
            _temporary_path: None,
        }
    }

    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    pub(crate) fn from_temporary(path: TempPath) -> Self {
        Self {
            path: path.to_path_buf(),
            _temporary_path: Some(path),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("The screenshot request failed: {0}")]
    Portal(String),
    #[error("The screenshot portal returned an invalid file location: {0}")]
    InvalidLocation(String),
    #[cfg(target_os = "macos")]
    #[error("No display is available for screen capture on {platform}")]
    NoDisplay { platform: &'static str },
    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    #[error(
        "CaptureVault cannot capture a selected area on {platform} yet. Use full-display capture instead."
    )]
    AreaSelectionUnavailable { platform: &'static str },
    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    #[error("CaptureVault could not create a temporary screenshot file: {source}")]
    TemporaryFile {
        #[source]
        source: std::io::Error,
    },
    #[cfg(any(target_os = "macos", target_os = "windows", test))]
    #[error("Native screen capture failed on {platform}: {message}")]
    Native {
        platform: &'static str,
        message: String,
    },
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    #[error("Screen capture is not implemented on this platform yet")]
    UnsupportedPlatform,
}

/// Allocate a temporary PNG path that will be deleted with the returned
/// [`CapturedImage`]. The empty file is intentional: native image writers
/// truncate it when producing the capture.
#[cfg(any(target_os = "macos", target_os = "windows", test))]
pub(super) fn temporary_png_path() -> Result<TempPath, CaptureError> {
    tempfile::Builder::new()
        .prefix("capture-vault-")
        .suffix(".png")
        .tempfile()
        .map(|file| file.into_temp_path())
        .map_err(|source| CaptureError::TemporaryFile { source })
}

#[cfg(target_os = "linux")]
fn active_provider() -> &'static dyn CaptureProvider {
    static PROVIDER: linux::LinuxCaptureProvider = linux::LinuxCaptureProvider;
    &PROVIDER
}

#[cfg(target_os = "macos")]
fn active_provider() -> &'static dyn CaptureProvider {
    static PROVIDER: macos::MacOsCaptureProvider = macos::MacOsCaptureProvider;
    &PROVIDER
}

#[cfg(target_os = "windows")]
fn active_provider() -> &'static dyn CaptureProvider {
    static PROVIDER: windows::WindowsCaptureProvider = windows::WindowsCaptureProvider;
    &PROVIDER
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn active_provider() -> &'static dyn CaptureProvider {
    static PROVIDER: unsupported::UnsupportedCaptureProvider =
        unsupported::UnsupportedCaptureProvider;
    &PROVIDER
}

pub async fn take_screenshot(mode: CaptureMode) -> Result<CapturedImage, CaptureError> {
    active_provider().capture(mode).await
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{CaptureError, CapturedImage, temporary_png_path};

    #[test]
    fn temporary_native_output_is_removed_when_the_artifact_is_dropped() {
        let path = temporary_png_path().expect("allocate temporary file");
        let file_path = path.to_path_buf();
        fs::write(&file_path, b"native screenshot").expect("write temporary image");

        let artifact = CapturedImage::from_temporary(path);
        assert_eq!(artifact.path(), file_path);
        drop(artifact);

        assert!(!file_path.exists());
    }

    #[test]
    fn portal_output_is_not_deleted_by_capturevault() {
        let path = tempfile::NamedTempFile::new()
            .expect("create portal fixture")
            .into_temp_path();
        let file_path = path.to_path_buf();
        let _artifact = CapturedImage::from_portal(file_path.clone());

        assert!(file_path.exists());
    }

    #[test]
    fn unsupported_area_requests_are_explicit() {
        let error = CaptureError::AreaSelectionUnavailable {
            platform: "Windows",
        };

        assert_eq!(
            error.to_string(),
            "CaptureVault cannot capture a selected area on Windows yet. Use full-display capture instead."
        );
    }

    #[test]
    fn native_errors_identify_the_platform() {
        let error = CaptureError::Native {
            platform: "macOS",
            message: "screen recording permission was denied".into(),
        };

        assert_eq!(
            error.to_string(),
            "Native screen capture failed on macOS: screen recording permission was denied"
        );
    }
}
