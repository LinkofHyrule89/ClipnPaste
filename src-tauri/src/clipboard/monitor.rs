use crate::clipboard::types::ClipItemType;
use crate::db::Database;
use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::ImageEncoder;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct ClipboardMonitor {
    stop: Arc<AtomicBool>,
}

impl ClipboardMonitor {
    pub fn start(db: Arc<Mutex<Database>>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();

        thread::spawn(move || {
            let mut last_hash = String::new();
            while !stop_flag.load(Ordering::Relaxed) {
                if let Ok(mut clipboard) = Clipboard::new() {
                    if let Some((item_type, content, preview, hash)) = read_clipboard(&mut clipboard)
                    {
                        if hash != last_hash {
                            last_hash = hash;
                            if let Ok(db) = db.lock() {
                                let _ = db.insert_item(item_type, &content, &preview);
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(400));
            }
        });

        Self { stop }
    }
}

impl Drop for ClipboardMonitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn read_clipboard(clipboard: &mut Clipboard) -> Option<(ClipItemType, String, String, String)> {
    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            let preview = preview_text(&text);
            let hash = hash_bytes(text.as_bytes());
            return Some((ClipItemType::Text, text, preview, hash));
        }
    }

    if let Ok(image) = clipboard.get_image() {
        if let Some((content, preview, hash)) = encode_image(image) {
            return Some((ClipItemType::Image, content, preview, hash));
        }
    }

    None
}

fn preview_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().take(3).collect();
    let mut preview = lines.join("\n");
    if text.lines().count() > 3 {
        preview.push_str("…");
    }
    if preview.len() > 240 {
        preview.truncate(240);
        preview.push('…');
    }
    preview
}

fn encode_image(image: ImageData) -> Option<(String, String, String)> {
    let rgba = image.rgba;
    let width = image.width as u32;
    let height = image.height as u32;
    if width == 0 || height == 0 {
        return None;
    }

    let img = image::RgbaImage::from_raw(width, height, rgba)?;
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
    let preview = content.clone();
    let hash = hash_bytes(png_bytes.as_slice());
    Some((content, preview, hash))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

pub fn write_text(text: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(text).map_err(|e| e.to_string())
}

pub fn write_image_png(png_bytes: &[u8]) -> Result<(), String> {
    let img = image::load_from_memory(png_bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let image_data = ImageData {
        width: width as usize,
        height: height as usize,
        rgba: rgba.into_raw(),
    };
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard
        .set_image(image_data)
        .map_err(|e| e.to_string())
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