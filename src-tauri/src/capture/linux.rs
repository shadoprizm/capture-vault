use ashpd::desktop::screenshot::{AvailableTargets, Screenshot, ScreenshotProxy};

use super::{CaptureError, CaptureFuture, CaptureProvider, CapturedImage};
use crate::models::CaptureMode;

pub(super) struct LinuxCaptureProvider;

impl CaptureProvider for LinuxCaptureProvider {
    fn capture(&self, mode: CaptureMode) -> CaptureFuture {
        Box::pin(async move {
            let target = match mode {
                CaptureMode::Area => AvailableTargets::Area,
                CaptureMode::Screen => AvailableTargets::Screen,
            };

            // Version 3 adds explicit targets. Older portals still support the
            // interactive option, which gives area captures a safe,
            // compositor-owned fallback.
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

            let path = uri
                .to_file_path()
                .map_err(|_| CaptureError::InvalidLocation(uri.to_string()))?;

            Ok(CapturedImage::from_portal(path))
        })
    }
}
