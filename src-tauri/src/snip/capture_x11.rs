use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::ImageEncoder;
use thiserror::Error;
use xcap::{Monitor, Window};

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture failed: {0}")]
    Message(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
    /// Path written under Screenshots on auto-save (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_path: Option<String>,
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

/// Capture a screen region.
///
/// `x`, `y`, `width`, and `height` are in **physical X11 root coordinates**
/// (same space as Tauri `outerPosition` / CSS × device scale).
///
/// xcap exposes monitor geometry in *logical* pixels (÷ `scale_factor` from
/// Xft.dpi) while `capture_image()` returns *physical* pixels. We convert
/// bounds with `scale_factor` and crop against the real image size.
pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<CaptureResult, CaptureError> {
    if width == 0 || height == 0 {
        return Err(CaptureError::Message("Invalid region size".into()));
    }

    let monitors = Monitor::all().map_err(|e| CaptureError::Message(e.to_string()))?;
    if monitors.is_empty() {
        return Err(CaptureError::Message("No monitor found".into()));
    }

    let monitor = find_monitor_for_physical_point(&monitors, x, y)
        .cloned()
        .or_else(|| monitors.iter().find(|m| m.is_primary()).cloned())
        .or_else(|| monitors.into_iter().next())
        .ok_or_else(|| CaptureError::Message("No monitor found".into()))?;

    let scale = monitor.scale_factor().max(0.01);
    let mx = (monitor.x() as f32 * scale).round() as i32;
    let my = (monitor.y() as f32 * scale).round() as i32;
    let mw = ((monitor.width() as f32 * scale).round() as i32).max(1);
    let mh = ((monitor.height() as f32 * scale).round() as i32).max(1);

    let image = monitor
        .capture_image()
        .map_err(|e| CaptureError::Message(e.to_string()))?;
    let (iw, ih) = image.dimensions();
    if iw == 0 || ih == 0 {
        return Err(CaptureError::Message("Empty capture".into()));
    }

    let (rx, ry, rw, rh) = map_and_clamp_region(x, y, width, height, mx, my, mw, mh, iw, ih)
        .map_err(CaptureError::Message)?;

    let cropped =
        image::imageops::crop_imm(&image, rx, ry, rw, rh).to_image();
    encode_image(&cropped)
}

/// Map a physical-screen region onto monitor image pixels and clamp to bounds.
///
/// Returns `(x, y, width, height)` in image pixel space.
pub fn map_and_clamp_region(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    mx: i32,
    my: i32,
    mw: i32,
    mh: i32,
    iw: u32,
    ih: u32,
) -> Result<(u32, u32, u32, u32), String> {
    if width == 0 || height == 0 || mw <= 0 || mh <= 0 || iw == 0 || ih == 0 {
        return Err("Invalid region size".into());
    }

    let sx = iw as f32 / mw as f32;
    let sy = ih as f32 / mh as f32;

    let rel_x = (x - mx) as f32;
    let rel_y = (y - my) as f32;
    let mut rx = (rel_x * sx).round() as i32;
    let mut ry = (rel_y * sy).round() as i32;
    let mut rw = (width as f32 * sx).round() as i32;
    let mut rh = (height as f32 * sy).round() as i32;

    if rx < 0 {
        rw += rx;
        rx = 0;
    }
    if ry < 0 {
        rh += ry;
        ry = 0;
    }
    if rx >= iw as i32 || ry >= ih as i32 || rw <= 0 || rh <= 0 {
        return Err("Region outside monitor".into());
    }
    rw = rw.min(iw as i32 - rx);
    rh = rh.min(ih as i32 - ry);
    if rw <= 0 || rh <= 0 {
        return Err("Region outside monitor".into());
    }

    Ok((rx as u32, ry as u32, rw as u32, rh as u32))
}

fn find_monitor_for_physical_point(monitors: &[Monitor], x: i32, y: i32) -> Option<&Monitor> {
    monitors.iter().find(|m| {
        let scale = m.scale_factor().max(0.01);
        let mx = (m.x() as f32 * scale).round() as i32;
        let my = (m.y() as f32 * scale).round() as i32;
        let mw = ((m.width() as f32 * scale).round() as i32).max(1);
        let mh = ((m.height() as f32 * scale).round() as i32).max(1);
        x >= mx && y >= my && x < mx + mw && y < my + mh
    })
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
        saved_path: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_identity_scale_full_monitor() {
        // Region covers whole 100x50 physical monitor → full 100x50 image.
        let (rx, ry, rw, rh) =
            map_and_clamp_region(0, 0, 100, 50, 0, 0, 100, 50, 100, 50).unwrap();
        assert_eq!((rx, ry, rw, rh), (0, 0, 100, 50));
    }

    #[test]
    fn map_2x_scale_physical_to_image() {
        // Logical 100x100 at scale 2 → image 200x200; region 10,10 20x20 physical
        // maps 1:1 if mw/mh already physical (mw=200).
        let (rx, ry, rw, rh) =
            map_and_clamp_region(10, 10, 20, 20, 0, 0, 200, 200, 200, 200).unwrap();
        assert_eq!((rx, ry, rw, rh), (10, 10, 20, 20));
    }

    #[test]
    fn map_clamps_partial_overflow() {
        let (rx, ry, rw, rh) =
            map_and_clamp_region(90, 90, 50, 50, 0, 0, 100, 100, 100, 100).unwrap();
        assert_eq!(rx, 90);
        assert_eq!(ry, 90);
        assert_eq!(rw, 10);
        assert_eq!(rh, 10);
    }

    #[test]
    fn map_rejects_fully_outside() {
        assert!(map_and_clamp_region(200, 200, 10, 10, 0, 0, 100, 100, 100, 100).is_err());
    }

    #[test]
    fn map_rejects_zero_size() {
        assert!(map_and_clamp_region(0, 0, 0, 10, 0, 0, 100, 100, 100, 100).is_err());
        assert!(map_and_clamp_region(0, 0, 10, 0, 0, 0, 100, 100, 100, 100).is_err());
    }

    #[test]
    fn map_negative_origin_clamped() {
        let (rx, ry, rw, rh) =
            map_and_clamp_region(-10, -5, 30, 20, 0, 0, 100, 100, 100, 100).unwrap();
        assert_eq!(rx, 0);
        assert_eq!(ry, 0);
        assert_eq!(rw, 20);
        assert_eq!(rh, 15);
    }
}