//! Native macOS screen capture.
//!
//! `SCScreenshotManager` provides a genuine single-frame capture on macOS 14+
//! without creating a recording stream. Area capture delegates only the
//! selection interaction to macOS' built-in `screencapture` picker, which keeps
//! the familiar crosshair, Escape-to-cancel behavior, and multi-display support.

use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicBool, Ordering},
};

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
                CaptureMode::Area => tauri::async_runtime::spawn_blocking(capture_selected_area)
                    .await
                    .map_err(|error| CaptureError::Native {
                        platform: "macOS",
                        message: format!(
                            "the native area-selection worker stopped unexpectedly: {error}"
                        ),
                    })?,
            }
        })
    }
}

fn capture_first_display() -> Result<CapturedImage, CaptureError> {
    check_permission()?;
    let content = SCShareableContent::get().map_err(native_error)?;
    let displays = content.displays();
    let main_display_id = main_display_id();
    let display = displays
        .iter()
        .find(|display| display.display_id() == main_display_id)
        .or_else(|| displays.first())
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

fn capture_selected_area() -> Result<CapturedImage, CaptureError> {
    check_permission()?;
    capture_with_screencapture(&["-i", "-s", "-x", "-t", "png"], true)
}

#[cfg(debug_assertions)]
pub(super) fn capture_test_area() -> Result<CapturedImage, CaptureError> {
    capture_with_screencapture(&["-R0,0,256,256", "-x", "-t", "png"], false)
}

fn capture_with_screencapture(
    arguments: &[&str],
    empty_output_is_cancelled: bool,
) -> Result<CapturedImage, CaptureError> {
    let path = temporary_png_path()?;
    let output = Command::new("/usr/sbin/screencapture")
        .args(arguments)
        .arg(path.as_os_str())
        .output()
        .map_err(|error| CaptureError::Native {
            platform: "macOS",
            message: format!("could not launch the macOS area picker: {error}"),
        })?;

    let captured = fs::metadata(path.to_path_buf())
        .map(|metadata| metadata.len() > 0)
        .unwrap_or(false);

    if captured {
        Ok(CapturedImage::from_temporary(path))
    } else if unsafe { !CGPreflightScreenCaptureAccess() } {
        Err(CaptureError::PermissionDenied)
    } else if empty_output_is_cancelled && output.stderr.is_empty() {
        Err(CaptureError::Cancelled)
    } else {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(CaptureError::Native {
            platform: "macOS",
            message: if detail.is_empty() {
                format!(
                    "the macOS area picker exited with {} without saving an image",
                    output.status
                )
            } else {
                format!("the macOS area picker failed: {detail}")
            },
        })
    }
}

fn check_permission() -> Result<(), CaptureError> {
    // TCC can retain an enabled-looking entry for an older signature. Ask the
    // OS for the current process's actual access before launching the picker.
    static REQUESTED_THIS_RUN: AtomicBool = AtomicBool::new(false);
    let allowed = unsafe { CGPreflightScreenCaptureAccess() }
        || (!REQUESTED_THIS_RUN.swap(true, Ordering::AcqRel)
            && unsafe { CGRequestScreenCaptureAccess() });
    if allowed {
        Ok(())
    } else {
        Err(CaptureError::PermissionDenied)
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

fn main_display_id() -> u32 {
    // SAFETY: CGMainDisplayID takes no arguments and returns the active main
    // display's stable CoreGraphics identifier.
    unsafe { CGMainDisplayID() }
}

fn native_error(error: impl std::fmt::Display) -> CaptureError {
    let message = error.to_string();
    if message.to_ascii_lowercase().contains("declined tcc") {
        CaptureError::PermissionDenied
    } else {
        CaptureError::Native {
            platform: "macOS",
            message,
        }
    }
}
