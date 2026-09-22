//! ScreenCaptureKit implementation for full-display still captures.
//!
//! `SCScreenshotManager` provides a genuine single-frame capture on macOS 14+
//! without creating a recording stream. It has no arbitrary region-selection
//! UI, so `CaptureMode::Area` intentionally reports an actionable error.

use std::fs;

use screencapturekit::{
    screenshot_manager::{CGImageExt, SCScreenshotManager},
    shareable_content::SCShareableContent,
    stream::{
        configuration::{PixelFormat, SCStreamConfiguration},
        content_filter::SCContentFilter,
    },
};

use super::{CaptureError, CaptureFuture, CaptureProvider, CapturedImage, temporary_png_path};
use crate::models::CaptureMode;

pub(super) struct MacOsCaptureProvider;

impl CaptureProvider for MacOsCaptureProvider {
    fn capture(&self, mode: CaptureMode) -> CaptureFuture {
        Box::pin(async move {
            match mode {
                CaptureMode::Screen => tauri::async_runtime::spawn_blocking(capture_first_display)
                    .await
                    .map_err(|error| CaptureError::Native {
                        platform: "macOS",
                        message: format!(
                            "the ScreenCaptureKit worker stopped unexpectedly: {error}"
                        ),
                    })?,
                CaptureMode::Area => {
                    Err(CaptureError::AreaSelectionUnavailable { platform: "macOS" })
                }
            }
        })
    }
}

fn capture_first_display() -> Result<CapturedImage, CaptureError> {
    let content = SCShareableContent::get().map_err(native_error)?;
    let display = content
        .displays()
        .into_iter()
        .next()
        .ok_or(CaptureError::NoDisplay { platform: "macOS" })?;

    let filter = SCContentFilter::create()
        .with_display(&display)
        .with_excluding_windows(&[])
        .build();
    let configuration = SCStreamConfiguration::new()
        .with_width(display.width())
        .with_height(display.height())
        .with_pixel_format(PixelFormat::BGRA);
    let screenshot =
        SCScreenshotManager::capture_image(&filter, &configuration).map_err(native_error)?;

    // Use the existing PNG codec rather than a lossy path conversion for the
    // temporary file. `rgba_data` also normalizes CoreGraphics' native BGRA
    // representation before it reaches the shared image store.
    let width = u32::try_from(screenshot.width()).map_err(|error| CaptureError::Native {
        platform: "macOS",
        message: format!("ScreenCaptureKit returned an unsupported image width: {error}"),
    })?;
    let height = u32::try_from(screenshot.height()).map_err(|error| CaptureError::Native {
        platform: "macOS",
        message: format!("ScreenCaptureKit returned an unsupported image height: {error}"),
    })?;
    let pixels = screenshot.rgba_data().map_err(native_error)?;
    let image =
        image::RgbaImage::from_raw(width, height, pixels).ok_or_else(|| CaptureError::Native {
            platform: "macOS",
            message: "ScreenCaptureKit returned a pixel buffer with unexpected dimensions".into(),
        })?;

    let path = temporary_png_path()?;
    if let Err(error) = image.save_with_format(&path, image::ImageFormat::Png) {
        let _ = fs::remove_file(&path);
        return Err(CaptureError::Native {
            platform: "macOS",
            message: format!("could not encode the ScreenCaptureKit frame as PNG: {error}"),
        });
    }

    Ok(CapturedImage::from_temporary(path))
}

fn native_error(error: impl std::fmt::Display) -> CaptureError {
    CaptureError::Native {
        platform: "macOS",
        message: error.to_string(),
    }
}
