use crate::db::Database;
use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use gtk::glib::{self, ControlFlow};
use image::imageops::FilterType;
use image::ImageEncoder;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use super::types::ClipItemType;

/// Frontend listens for this and reloads history (payload is unused unit).
pub const HISTORY_CHANGED: &str = "history-changed";

const THUMB_MAX_EDGE: u32 = 240;
const POLL_MS: u64 = 400;

/// Shared last-seen clipboard content hash so app writes skip re-processing.
static LAST_HASH: OnceLock<Arc<Mutex<String>>> = OnceLock::new();

fn last_hash() -> Arc<Mutex<String>> {
    LAST_HASH
        .get_or_init(|| Arc::new(Mutex::new(String::new())))
        .clone()
}

pub fn mark_seen_hash(hash: String) {
    if let Ok(mut guard) = last_hash().lock() {
        *guard = hash;
    }
}

pub fn emit_history_changed(app: &AppHandle) {
    let _ = app.emit(HISTORY_CHANGED, ());
}

pub struct ClipboardMonitor;

impl ClipboardMonitor {
    pub fn start(app: AppHandle, db: Arc<Mutex<Database>>) -> Self {
        let hash_state = last_hash();

        glib::timeout_add_local(Duration::from_millis(POLL_MS), move || {
            poll_once(&app, &db, &hash_state);
            ControlFlow::Continue
        });

        Self
    }
}

fn poll_once(app: &AppHandle, db: &Arc<Mutex<Database>>, hash_state: &Arc<Mutex<String>>) {
    let Ok(mut clipboard) = Clipboard::new() else {
        return;
    };

    // Prefer image: image copies often include incidental text (URL, path, HTML).
    if let Ok(image) = clipboard.get_image() {
        if let Some(probe) = probe_image(image) {
            {
                let last = hash_state.lock().expect("hash lock");
                if probe.hash == *last {
                    return;
                }
            }
            // Encode only when clipboard content actually changed (lock not held).
            let Some((content, preview)) = encode_probed_image(probe.width, probe.height, probe.raw)
            else {
                return;
            };
            {
                let mut last = hash_state.lock().expect("hash lock");
                // Another writer may have updated; still mark this hash as seen.
                *last = probe.hash;
            }

            let inserted = {
                let Ok(db) = db.lock() else {
                    return;
                };
                db.insert_item(ClipItemType::Image, &content, &preview)
                    .ok()
                    .flatten()
            };
            if inserted.is_some() {
                emit_history_changed(app);
            }
            return;
        }
    }

    if let Ok(text) = clipboard.get_text() {
        if text.is_empty() {
            return;
        }
        let hash = hash_bytes(text.as_bytes());
        {
            let last = hash_state.lock().expect("hash lock");
            if hash == *last {
                return;
            }
        }
        {
            let mut last = hash_state.lock().expect("hash lock");
            *last = hash;
        }

        let preview = preview_text(&text);
        let inserted = {
            let Ok(db) = db.lock() else {
                return;
            };
            db.insert_item(ClipItemType::Text, &text, &preview)
                .ok()
                .flatten()
        };
        if inserted.is_some() {
            emit_history_changed(app);
        }
    }
}

struct ImageProbe {
    width: u32,
    height: u32,
    raw: Vec<u8>,
    hash: String,
}

fn probe_image(image: ImageData) -> Option<ImageProbe> {
    let width = image.width as u32;
    let height = image.height as u32;
    if width == 0 || height == 0 {
        return None;
    }
    let raw = image.bytes.into_owned();
    if raw.len() != (width as usize).saturating_mul(height as usize).saturating_mul(4) {
        // Unexpected buffer size — still try, but hash whatever we have.
    }
    let mut hasher = Sha256::new();
    hasher.update(width.to_le_bytes());
    hasher.update(height.to_le_bytes());
    hasher.update(&raw);
    let hash = format!("{:x}", hasher.finalize());
    Some(ImageProbe {
        width,
        height,
        raw,
        hash,
    })
}

fn encode_probed_image(width: u32, height: u32, raw: Vec<u8>) -> Option<(String, String)> {
    let img = image::RgbaImage::from_raw(width, height, raw)?;
    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            img.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;

    let b64 = STANDARD.encode(&png_bytes);
    let content = format!("data:image/png;base64,{b64}");
    let preview = make_thumbnail_data_url(&img).unwrap_or_else(|| content.clone());
    Some((content, preview))
}

pub fn preview_text(text: &str) -> String {
    const MAX_PREVIEW_BYTES: usize = 240;
    let lines: Vec<&str> = text.lines().take(3).collect();
    let mut preview = lines.join("\n");
    if text.lines().count() > 3 {
        preview.push('…');
    }
    if preview.len() > MAX_PREVIEW_BYTES {
        // Leave room for a single ellipsis character (3 UTF-8 bytes).
        let mut end = MAX_PREVIEW_BYTES.saturating_sub(3);
        while end > 0 && !preview.is_char_boundary(end) {
            end -= 1;
        }
        preview.truncate(end);
        preview.push('…');
    }
    preview
}

fn make_thumbnail_data_url(img: &image::RgbaImage) -> Option<String> {
    let (width, height) = img.dimensions();
    let max_edge = width.max(height);
    let thumb = if max_edge > THUMB_MAX_EDGE {
        let scale = THUMB_MAX_EDGE as f32 / max_edge as f32;
        let tw = ((width as f32) * scale).round().max(1.0) as u32;
        let th = ((height as f32) * scale).round().max(1.0) as u32;
        image::imageops::resize(img, tw, th, FilterType::Triangle)
    } else {
        img.clone()
    };

    let mut png_bytes = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            thumb.as_raw(),
            thumb.width(),
            thumb.height(),
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;
    let b64 = STANDARD.encode(&png_bytes);
    Some(format!("data:image/png;base64,{b64}"))
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

pub fn write_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    mark_seen_hash(hash_bytes(text.as_bytes()));
    Ok(())
}

pub fn write_image_png(png_bytes: &[u8]) -> Result<(), String> {
    let img = image::load_from_memory(png_bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let raw = rgba.into_raw();

    let mut hasher = Sha256::new();
    hasher.update(width.to_le_bytes());
    hasher.update(height.to_le_bytes());
    hasher.update(&raw);
    let hash = format!("{:x}", hasher.finalize());

    let image_data = ImageData {
        width: width as usize,
        height: height as usize,
        bytes: std::borrow::Cow::from(raw),
    };
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_image(image_data)
        .map_err(|e| e.to_string())?;
    mark_seen_hash(hash);
    Ok(())
}

pub fn write_item_to_clipboard(item_type: ClipItemType, content: &str) -> Result<(), String> {
    match item_type {
        ClipItemType::Text => write_text(content),
        ClipItemType::Image => {
            let payload = content
                .strip_prefix("data:image/png;base64,")
                .unwrap_or(content);
            let bytes = STANDARD.decode(payload).map_err(|e| e.to_string())?;
            write_image_png(&bytes)
        }
    }
}

/// Build full content + thumbnail preview from raw PNG bytes (snips).
pub fn image_content_and_preview(png_bytes: &[u8]) -> Result<(String, String), String> {
    let img = image::load_from_memory(png_bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let b64 = STANDARD.encode(png_bytes);
    let content = format!("data:image/png;base64,{b64}");
    let preview = make_thumbnail_data_url(&rgba).unwrap_or_else(|| content.clone());
    Ok((content, preview))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;

    fn tiny_png() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));
        let mut png = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png);
        encoder
            .write_image(img.as_raw(), 2, 2, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    #[test]
    fn preview_text_truncates_lines() {
        let text = "a\nb\nc\nd\ne";
        let preview = preview_text(text);
        assert!(preview.starts_with("a\nb\nc"));
        assert!(preview.ends_with('…'));
        // First three lines only; fourth line body is not included as its own line.
        assert!(!preview.contains("\nd"));
        assert_eq!(preview.lines().count(), 3);
    }

    #[test]
    fn preview_text_truncates_length() {
        let text = "x".repeat(400);
        let preview = preview_text(&text);
        assert!(preview.len() <= 241);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn preview_text_short_unchanged() {
        assert_eq!(preview_text("hi"), "hi");
    }

    #[test]
    fn hash_bytes_stable() {
        assert_eq!(hash_bytes(b"abc"), hash_bytes(b"abc"));
        assert_ne!(hash_bytes(b"abc"), hash_bytes(b"abd"));
    }

    #[test]
    fn image_content_and_preview_roundtrip() {
        let png = tiny_png();
        let (content, preview) = image_content_and_preview(&png).unwrap();
        assert!(content.starts_with("data:image/png;base64,"));
        assert!(preview.starts_with("data:image/png;base64,"));
        // Full content includes the original payload; preview is a (possibly same-size) thumb.
        assert!(content.len() >= preview.len() || preview.len() > 0);
    }
}
