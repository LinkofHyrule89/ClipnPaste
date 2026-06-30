use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::ImageEncoder;
use thiserror::Error;
use xcap::{Monitor, Window};

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture failed: {0}")]
    Message(String),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub app_name: String,
}

pub fn list_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    let windows = Window::all().map_err(|e| CaptureError::Message(e.to_string()))?;
    let mut result = Vec::new();
    for window in windows {
        let title = window.title().unwrap_or_default();
        let app_name = window.app_name().unwrap_or_default();
        if title.is_empty() && app_name.is_empty() {
            continue;
        }
        if window.is_minimized().unwrap_or(false) {
            continue;
        }
        result.push(WindowInfo {
            id: window.id().map_err(|e| CaptureError::Message(e.to_string()))?,
            title,
            app_name,
        });
    }
    result.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(result)
}

pub fn capture_fullscreen() -> Result<CaptureResult, CaptureError> {
    let monitors = Monitor::all().map_err(|e| CaptureError::Message(e.to_string()))?;
    let monitor = monitors
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| Monitor::all().ok()?.into_iter().next())
        .ok_or_else(|| CaptureError::Message("No monitor found".into()))?;
    capture_monitor(&monitor)
}

pub fn capture_window(window_id: u32) -> Result<CaptureResult, CaptureError> {
    let windows = Window::all().map_err(|e| CaptureError::Message(e.to_string()))?;
    let window = windows
        .into_iter()
        .find(|w| w.id().ok() == Some(window_id))
        .ok_or_else(|| CaptureError::Message("Window not found".into()))?;
    let image = window.capture_image().map_err(|e| CaptureError::Message(e.to_string()))?;
    encode_image(&image)
}

pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<CaptureResult, CaptureError> {
    if width == 0 || height == 0 {
        return Err(CaptureError::Message("Invalid region size".into()));
    }

    let monitors = Monitor::all().map_err(|e| CaptureError::Message(e.to_string()))?;
    for monitor in monitors {
        let mx = monitor.x().map_err(|e| CaptureError::Message(e.to_string()))?;
        let my = monitor.y().map_err(|e| CaptureError::Message(e.to_string()))?;
        let mw = monitor.width().map_err(|e| CaptureError::Message(e.to_string()))? as i32;
        let mh = monitor.height().map_err(|e| CaptureError::Message(e.to_string()))? as i32;

        if x >= mx && y >= my && x < mx + mw && y < my + mh {
            let image = monitor
                .capture_image()
                .map_err(|e| CaptureError::Message(e.to_string()))?;
            let rx = (x - mx) as u32;
            let ry = (y - my) as u32;
            let crop_w = width.min(mw as u32 - rx);
            let crop_h = height.min(mh as u32 - ry);
            let cropped = image::imageops::crop_imm(&image, rx, ry, crop_w, crop_h).to_image();
            return encode_image(&cropped);
        }
    }

    Err(CaptureError::Message("Region outside monitors".into()))
}

fn capture_monitor(monitor: &Monitor) -> Result<CaptureResult, CaptureError> {
    let image = monitor
        .capture_image()
        .map_err(|e| CaptureError::Message(e.to_string()))?;
    encode_image(&image)
}

fn encode_image(image: &image::RgbaImage) -> Result<CaptureResult, CaptureError> {
    let (width, height) = image.dimensions();
    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            image.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| CaptureError::Message(e.to_string()))?;

    Ok(CaptureResult {
        png_base64: STANDARD.encode(png_bytes),
        width,
        height,
    })
}