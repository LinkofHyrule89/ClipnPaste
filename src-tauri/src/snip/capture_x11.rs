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
        let title = window.title().to_string();
        let app_name = window.app_name().to_string();
        if title.is_empty() && app_name.is_empty() {
            continue;
        }
        if window.is_minimized() {
            continue;
        }
        result.push(WindowInfo {
            id: window.id(),
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
        .find(|m| m.is_primary())
        .or_else(|| Monitor::all().ok()?.into_iter().next())
        .ok_or_else(|| CaptureError::Message("No monitor found".into()))?;
    capture_monitor(&monitor)
}

pub fn capture_window(window_id: u32) -> Result<CaptureResult, CaptureError> {
    let windows = Window::all().map_err(|e| CaptureError::Message(e.to_string()))?;
    let window = windows
        .into_iter()
        .find(|w| w.id() == window_id)
        .ok_or_else(|| CaptureError::Message("Window not found".into()))?;
    let image = window
        .capture_image()
        .map_err(|e| CaptureError::Message(e.to_string()))?;
    encode_image(&image)
}

pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<CaptureResult, CaptureError> {
    if width == 0 || height == 0 {
        return Err(CaptureError::Message("Invalid region size".into()));
    }

    let monitor = Monitor::from_point(x, y).map_err(|e| CaptureError::Message(e.to_string()))?;
    let image = monitor
        .capture_image()
        .map_err(|e| CaptureError::Message(e.to_string()))?;
    let mx = monitor.x();
    let my = monitor.y();
    let mw = monitor.width() as i32;
    let mh = monitor.height() as i32;

    if x < mx || y < my || x >= mx + mw || y >= my + mh {
        return Err(CaptureError::Message("Region outside monitor".into()));
    }

    let rx = (x - mx) as u32;
    let ry = (y - my) as u32;
    let crop_w = width.min(mw as u32 - rx);
    let crop_h = height.min(mh as u32 - ry);
    let cropped = image::imageops::crop_imm(&image, rx, ry, crop_w, crop_h).to_image();
    encode_image(&cropped)
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