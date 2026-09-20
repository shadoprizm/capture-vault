use std::path::PathBuf;

use thiserror::Error;

use crate::models::CaptureMode;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("The screenshot request failed: {0}")]
    Portal(String),
    #[error("The screenshot portal returned an invalid file location: {0}")]
    InvalidLocation(String),
    #[cfg(not(target_os = "linux"))]
    #[error("Screen capture is not implemented on this platform yet")]
    UnsupportedPlatform,
}

#[cfg(target_os = "linux")]
pub async fn take_screenshot(mode: CaptureMode) -> Result<PathBuf, CaptureError> {
    use ashpd::desktop::screenshot::{AvailableTargets, Screenshot, ScreenshotProxy};

    let target = match mode {
        CaptureMode::Area => AvailableTargets::Area,
        CaptureMode::Screen => AvailableTargets::Screen,
    };

    // Version 3 adds explicit targets. Older portals still support the interactive
    // option, which gives area captures a safe, compositor-owned fallback.
    let portal_version = ScreenshotProxy::new()
        .await
        .map(|proxy| proxy.version())
        .unwrap_or_default();

    let mut request = Screenshot::request()
        .modal(true)
        .interactive(matches!(mode, CaptureMode::Area));

    if portal_version >= 3 {
        request = request.target(target);
    }

    let response = request
        .send()
        .await
        .map_err(|error| CaptureError::Portal(error.to_string()))?
        .response()
        .map_err(|error| CaptureError::Portal(error.to_string()))?;

    let uri = url::Url::parse(response.uri().as_str())
        .map_err(|error| CaptureError::InvalidLocation(error.to_string()))?;

    uri.to_file_path()
        .map_err(|_| CaptureError::InvalidLocation(uri.to_string()))
}

#[cfg(not(target_os = "linux"))]
pub async fn take_screenshot(_mode: CaptureMode) -> Result<PathBuf, CaptureError> {
    Err(CaptureError::UnsupportedPlatform)
}
