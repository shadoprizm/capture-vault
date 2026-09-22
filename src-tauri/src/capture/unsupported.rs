use super::{CaptureError, CaptureFuture, CaptureProvider};
use crate::models::CaptureMode;

pub(super) struct UnsupportedCaptureProvider;

impl CaptureProvider for UnsupportedCaptureProvider {
    fn capture(&self, _mode: CaptureMode) -> CaptureFuture {
        Box::pin(async { Err(CaptureError::UnsupportedPlatform) })
    }
}
