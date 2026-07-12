use crate::db::Database;
use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use gtk::glib::{self, ControlFlow};
use image::imageops::FilterType;
use image::ImageEncoder;
use sha2::{Digest, Sha256}; // Digest used in probe_image
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use super::policy::{choose_capture, hash_bytes, CaptureKind};
use super::types::ClipItemType;

/// Frontend listens for this and reloads history (payload is unused unit).
pub const HISTORY_CHANGED: &str = "history-changed";

const THUMB_MAX_EDGE: u32 = 240;
const POLL_MS: u64 = 400;

/// Separate gates for text vs image. A single shared hash caused new text
/// copies to be ignored when X11 still exposed a stale image target.
#[derive(Default)]
struct SeenHashes {
    text: String,
    image: String,
}

static SEEN: OnceLock<Arc<Mutex<SeenHashes>>> = OnceLock::new();

fn seen() -> Arc<Mutex<SeenHashes>> {
    SEEN.get_or_init(|| Arc::new(Mutex::new(SeenHashes::default())))
        .clone()
}

/// Mark whatever is currently on the system clipboard as already processed,
/// without writing to history. Used after Clear all / delete so the live
/// clipboard is not immediately re-inserted, while a *new* copy still is.
pub fn mark_live_clipboard_as_seen() {
    let Ok(mut clipboard) = Clipboard::new() else {
        return;
    };
    let mut text_hash = String::new();
    let mut image_hash = String::new();

    if let Ok(image) = clipboard.get_image() {
        if let Some(probe) = probe_image(image) {
            image_hash = probe.hash;
        }
    }
    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            text_hash = hash_bytes(text.as_bytes());
        }
    }

    if let Ok(mut guard) = seen().lock() {
        if !text_hash.is_empty() {
            guard.text = text_hash;
        }
        if !image_hash.is_empty() {
            guard.image = image_hash;
        }
    }
}

pub fn emit_history_changed(app: &AppHandle) {
    let _ = app.emit(HISTORY_CHANGED, ());
}

pub struct ClipboardMonitor;

impl ClipboardMonitor {
    pub fn start(app: AppHandle, db: Arc<Mutex<Database>>) -> Self {
        glib::timeout_add_local(Duration::from_millis(POLL_MS), move || {
            poll_once(&app, &db);
            ControlFlow::Continue
        });

        Self
    }
}

fn poll_once(app: &AppHandle, db: &Arc<Mutex<Database>>) {
    let Ok(mut clipboard) = Clipboard::new() else {
        return;
    };

    // Snapshot both formats. On X11, an old image target often remains after
    // a new text copy — we must not ignore text when only the image hash matches.
    let image = clipboard.get_image().ok().and_then(probe_image);
    let text = clipboard.get_text().ok().filter(|t| !t.is_empty());

    let text_hash = text.as_ref().map(|t| hash_bytes(t.as_bytes()));
    let image_hash = image.as_ref().map(|p| p.hash.clone());

    let kind = {
        let seen_state = seen();
        let guard = seen_state.lock().expect("seen lock");
        choose_capture(
            text.as_deref(),
            text_hash.as_deref(),
            image_hash.as_deref(),
            &guard.text,
            &guard.image,
        )
    };

    match kind {
        CaptureKind::None => {}
        CaptureKind::Image => {
            let Some(probe) = image else {
                return;
            };
            let hash = probe.hash.clone();
            let Some((content, preview)) =
                encode_probed_image(probe.width, probe.height, probe.raw)
            else {
                return;
            };
            let inserted = {
                let Ok(db) = db.lock() else {
                    return;
                };
                db.insert_item(ClipItemType::Image, &content, &preview)
                    .ok()
                    .flatten()
            };
            if inserted.is_some() {
                if let Ok(mut guard) = seen().lock() {
                    guard.image = hash;
                    if let Some(th) = text_hash {
                        guard.text = th;
                    }
                }
                emit_history_changed(app);
            }
        }
        CaptureKind::Text => {
            let Some(text) = text else {
                return;
            };
            let hash = text_hash.expect("text capture implies hash");
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
                if let Ok(mut guard) = seen().lock() {
                    guard.text = hash;
                    if let Some(ih) = image_hash {
                        guard.image = ih;
                    }
                }
                emit_history_changed(app);
            }
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

pub fn write_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    let th = hash_bytes(text.as_bytes());
    if let Ok(mut guard) = seen().lock() {
        guard.text = th;
        // App text write clears image intent for our gate.
        guard.image.clear();
    }
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
    if let Ok(mut guard) = seen().lock() {
        guard.image = hash;
        guard.text.clear();
    }
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
    fn image_content_and_preview_roundtrip() {
        let png = tiny_png();
        let (content, preview) = image_content_and_preview(&png).unwrap();
        assert!(content.starts_with("data:image/png;base64,"));
        assert!(preview.starts_with("data:image/png;base64,"));
        assert!(content.len() >= preview.len() || !preview.is_empty());
    }

    /// Skin-tone emoji are multiple Unicode scalars; clipboard write must keep them all.
    /// (What the target app *renders* may still look yellow if it ignores modifiers.)
    #[test]
    fn dark_skin_tone_emoji_utf8_payload_is_not_yellow_only() {
        let yellow = "\u{1F44D}"; // 👍 default / "yellow"
        let dark = "\u{1F44D}\u{1F3FF}"; // 👍🏿
        assert_ne!(yellow, dark);
        assert_eq!(yellow.chars().count(), 1);
        assert_eq!(dark.chars().count(), 2);

        let yellow_cps: Vec<u32> = yellow.chars().map(|c| c as u32).collect();
        let dark_cps: Vec<u32> = dark.chars().map(|c| c as u32).collect();
        assert_eq!(yellow_cps, vec![0x1F_44D]);
        assert_eq!(dark_cps, vec![0x1F_44D, 0x1F_3FF]);

        // Same bytes write_text() would put on the clipboard.
        let bytes = dark.as_bytes();
        assert!(bytes.len() > yellow.as_bytes().len());
        let roundtrip = std::str::from_utf8(bytes).unwrap();
        assert_eq!(roundtrip, dark);
        assert_ne!(roundtrip, yellow);
    }
}
