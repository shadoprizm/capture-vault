//! Windows Graphics Capture implementation for primary-monitor still captures.
//!
//! Windows Graphics Capture captures a monitor or window, not an arbitrary
//! rectangle. The app deliberately does not crop a full-monitor image when an
//! area was requested: that would pretend a selection happened when it did not.

use std::{fs, path::PathBuf};

use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    encoder::ImageFormat,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
        MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
    },
};

use super::{CaptureError, CaptureFuture, CaptureProvider, CapturedImage, temporary_png_path};
use crate::models::CaptureMode;

pub(super) struct WindowsCaptureProvider;

impl CaptureProvider for WindowsCaptureProvider {
    fn capture(&self, mode: CaptureMode) -> CaptureFuture {
        Box::pin(async move {
            match mode {
                CaptureMode::Screen => {
                    tauri::async_runtime::spawn_blocking(capture_primary_monitor)
                        .await
                        .map_err(|error| CaptureError::Native {
                            platform: "Windows",
                            message: format!(
                                "the Windows Graphics Capture worker stopped unexpectedly: {error}"
                            ),
                        })?
                }
                CaptureMode::Area => Err(CaptureError::AreaSelectionUnavailable {
                    platform: "Windows",
                }),
            }
        })
    }
}

fn capture_primary_monitor() -> Result<CapturedImage, CaptureError> {
    let path = temporary_png_path()?;
    let destination = path.to_path_buf();
    let monitor = Monitor::primary().map_err(native_error)?;
    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::Default,
        DrawBorderSettings::Default,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Rgba8,
        destination.clone(),
    );

    if let Err(error) = OneShotCapture::start(settings) {
        let _ = fs::remove_file(&path);
        return Err(native_error(error));
    }

    // `temporary_png_path` intentionally creates the destination up front, so
    // existence alone cannot prove the frame callback wrote a PNG.
    let wrote_png = fs::metadata(&destination)
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);
    if !wrote_png {
        return Err(CaptureError::Native {
            platform: "Windows",
            message: "Windows Graphics Capture ended without producing a PNG".into(),
        });
    }

    Ok(CapturedImage::from_temporary(path))
}

struct OneShotCapture {
    destination: PathBuf,
}

impl GraphicsCaptureApiHandler for OneShotCapture {
    type Flags = PathBuf;
    type Error = CaptureError;

    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            destination: context.flags,
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        frame
            .save_as_image(&self.destination, ImageFormat::Png)
            .map_err(native_error)?;

        // The first frame is the complete full-monitor screenshot requested by
        // this provider. Stopping here also releases the WGC yellow border
        // (where Windows chooses to draw one) as quickly as possible.
        capture_control.stop();
        Ok(())
    }
}

fn native_error(error: impl std::fmt::Display) -> CaptureError {
    CaptureError::Native {
        platform: "Windows",
        message: error.to_string(),
    }
}
