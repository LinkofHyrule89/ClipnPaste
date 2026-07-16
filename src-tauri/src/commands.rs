use crate::clipboard::{
    emit_history_changed,
    monitor::{
        image_content_and_preview, mark_live_clipboard_as_seen, preview_text,
        write_item_to_clipboard, write_text,
    },
    ClipItemSummary, ClipItemType,
};
use crate::db::Database;
use crate::focus_target;
use crate::paths::{decode_png_bounded, resolve_snip_save_path, sanitize_screenshot_filename};
use crate::settings::{self, AppSettings};
use crate::snip::{capture_fullscreen, capture_region, capture_window, list_windows, CaptureResult};
use crate::windows;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};
use x11rb::wrapper::ConnectionExt as _;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub focus_target: focus_target::FocusTargetStore,
    /// Most recent snip (for toast → editor).
    pub last_capture: Mutex<Option<CaptureResult>>,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(settings::get_locked(&state.settings))
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    settings::apply_settings(&app, &state.settings, settings)
}

#[tauri::command]
pub fn open_keyboard_shortcuts() -> Result<(), String> {
    settings::open_keyboard_shortcuts()
}

#[tauri::command]
pub fn show_settings(app: AppHandle) -> Result<(), String> {
    windows::show_settings_window(&app)
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Result<Vec<ClipItemSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_summaries().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pin_item(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_pinned(&id, true).map_err(|e| e.to_string())?;
    drop(db);
    emit_history_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn unpin_item(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_pinned(&id, false).map_err(|e| e.to_string())?;
    drop(db);
    emit_history_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_item(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_item(&id).map_err(|e| e.to_string())?;
    drop(db);
    // Don't re-ingest the live clipboard until it changes (avoids delete → flash back).
    mark_live_clipboard_as_seen();
    emit_history_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn clear_unpinned(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.clear_unpinned().map_err(|e| e.to_string())?;
    drop(db);
    // Keep list empty: treat current system clipboard as already seen.
    // A *new* copy (different content) still updates history on the next poll.
    mark_live_clipboard_as_seen();
    emit_history_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn get_item_content(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let item = db
        .get_item(&id)
        .map_err(|e| e.to_string())?
        .ok_or("Item not found")?;
    Ok(item.content)
}

#[tauri::command]
pub fn update_item_text(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    content: String,
) -> Result<(), String> {
    let preview = preview_text(&content);
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let updated = db
        .update_text_item(&id, &content, &preview)
        .map_err(|e| e.to_string())?;
    if updated.is_none() {
        return Err("Unable to update item (missing, not text, or too large)".into());
    }
    drop(db);
    emit_history_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn copy_text_to_clipboard(text: String) -> Result<(), String> {
    write_text(&text)
}

#[tauri::command]
pub fn paste_text_to_target(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> Result<(), String> {
    // Clipboard + Ctrl+V so multi-codepoint emoji (skin tones, ZWJ) stay intact.
    // xdotool "type" often mishandles those sequences as separate keystrokes.
    write_text(&text)?;
    windows::hide_clipboard_panel(&app);
    focus_target::paste_after_hide(
        &state.focus_target,
        focus_target::PasteMode::ClipboardPaste,
        None,
    );
    Ok(())
}

#[tauri::command]
pub fn paste_item_to_target(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let item = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_item(&id)
            .map_err(|e| e.to_string())?
            .ok_or("Item not found")?
    };

    // Put selection on the system clipboard so Ctrl+V pastes it next.
    write_item_to_clipboard(item.item_type, &item.content)?;

    // Windows 11-style: selected history entry becomes the most recent.
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let _ = db.promote_item(&item.id).map_err(|e| e.to_string())?;
    }
    emit_history_changed(&app);

    windows::hide_clipboard_panel(&app);

    // Always Ctrl+V after writing clipboard — works for images and for
    // multi-scalar text (emoji skin tones) where keystroke "type" is unreliable.
    focus_target::paste_after_hide(
        &state.focus_target,
        focus_target::PasteMode::ClipboardPaste,
        None,
    );
    Ok(())
}

#[tauri::command]
pub fn copy_item_to_clipboard(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let item = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_item(&id)
            .map_err(|e| e.to_string())?
            .ok_or("Item not found")?
    };
    write_item_to_clipboard(item.item_type, &item.content)?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let _ = db.promote_item(&item.id).map_err(|e| e.to_string())?;
    }
    emit_history_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_capture_windows() -> Result<Vec<crate::snip::WindowInfo>, String> {
    list_windows().map_err(|e| e.to_string())
}

/// Hide snip UI and wait for the compositor so GetImage does not include the
/// selection chrome / dim overlay.
fn prepare_for_screen_capture(app: &AppHandle) {
    windows::hide_snip_overlay(app);
    windows::hide_snip_toolbar(app);

    // Flush X11 so unmap requests are processed before we sample the root.
    if let Ok((conn, _)) = x11rb::rust_connection::RustConnection::connect(None) {
        let _ = conn.sync();
    }

    // Cinnamon/Muffin needs a beat after unmap before the desktop is painted.
    std::thread::sleep(std::time::Duration::from_millis(180));
}

#[tauri::command]
pub fn snip_fullscreen(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CaptureResult, String> {
    prepare_for_screen_capture(&app);
    let result = capture_fullscreen().map_err(|e| e.to_string())?;
    finalize_snip(&app, &state, result)
}

#[tauri::command]
pub fn snip_window(
    app: AppHandle,
    state: State<'_, AppState>,
    window_id: u32,
) -> Result<CaptureResult, String> {
    prepare_for_screen_capture(&app);
    let result = capture_window(window_id).map_err(|e| e.to_string())?;
    finalize_snip(&app, &state, result)
}

#[tauri::command]
pub fn snip_region(
    app: AppHandle,
    state: State<'_, AppState>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<CaptureResult, String> {
    prepare_for_screen_capture(&app);
    let result = capture_region(x, y, width, height).map_err(|e| e.to_string())?;
    finalize_snip(&app, &state, result)
}

#[tauri::command]
pub fn copy_png_to_clipboard(
    app: AppHandle,
    state: State<'_, AppState>,
    png_base64: String,
) -> Result<(), String> {
    let bytes = decode_png_bounded(&png_base64)?;
    crate::clipboard::monitor::write_image_png(&bytes)?;
    let (content, preview) = image_content_and_preview(&bytes)?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let _ = db.insert_item(ClipItemType::Image, &content, &preview);
    }
    emit_history_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn save_png(png_base64: String, filename: Option<String>) -> Result<String, String> {
    let bytes = decode_png_bounded(&png_base64)?;
    let name = match filename {
        Some(n) => sanitize_screenshot_filename(&n)?,
        None => default_screenshot_name(),
    };
    write_png_to_screenshots(&bytes, &name)
}

/// Update clipboard + history and write/overwrite the screenshot file.
///
/// Optional `path` may only target a file under the Screenshots directory
/// (basename or absolute path under that folder). Arbitrary paths are rejected.
#[tauri::command]
pub fn save_edited_snip(
    app: AppHandle,
    state: State<'_, AppState>,
    png_base64: String,
    path: Option<String>,
) -> Result<CaptureResult, String> {
    let bytes = decode_png_bounded(&png_base64)?;
    let (content, preview) = image_content_and_preview(&bytes)?;
    write_item_to_clipboard(ClipItemType::Image, &content)?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let _ = db.insert_item(ClipItemType::Image, &content, &preview);
    }
    emit_history_changed(&app);

    let folder = screenshot_dir()?;
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let dest = resolve_snip_save_path(&folder, path.as_deref(), &default_screenshot_name())?;
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    let saved_path = dest.to_string_lossy().to_string();

    let (width, height) = image_dimensions_from_png(&bytes)?;
    let result = CaptureResult {
        png_base64,
        width,
        height,
        saved_path: Some(saved_path),
    };
    if let Ok(mut last) = state.last_capture.lock() {
        *last = Some(result.clone());
    }
    Ok(result)
}

#[tauri::command]
pub fn get_last_snip_capture(
    state: State<'_, AppState>,
) -> Result<Option<CaptureResult>, String> {
    state
        .last_capture
        .lock()
        .map_err(|e| e.to_string())
        .map(|g| g.clone())
}

fn emit_editor_image(app: &AppHandle, payload: &CaptureResult) {
    // Prefer window-targeted emit (listener is in snip-editor webview).
    if let Some(window) = app.get_webview_window("snip-editor") {
        let _ = window.emit("editor-image", payload);
    }
    // Global emit as backup for any listener.
    let _ = app.emit("editor-image", payload);
}

/// Open the snip editor with optional capture payload (falls back to last snip).
#[tauri::command]
pub fn open_snip_editor(
    app: AppHandle,
    state: State<'_, AppState>,
    capture: Option<CaptureResult>,
) -> Result<(), String> {
    let payload = match capture {
        Some(c) => {
            if let Ok(mut last) = state.last_capture.lock() {
                *last = Some(c.clone());
            }
            Some(c)
        }
        None => state
            .last_capture
            .lock()
            .map_err(|e| e.to_string())?
            .clone(),
    };

    windows::show_snip_editor(&app)?;

    if let Some(payload) = payload {
        // Immediate emit, then retries so a late-mounted listener still gets the image.
        emit_editor_image(&app, &payload);
        let app2 = app.clone();
        std::thread::spawn(move || {
            for delay_ms in [150u64, 400, 800] {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                emit_editor_image(&app2, &payload);
            }
        });
    }
    Ok(())
}

/// Load a history image into the snip editor.
#[tauri::command]
pub fn open_snip_editor_from_history(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let item = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_item(&id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Item not found".to_string())?
    };
    if item.item_type != ClipItemType::Image {
        return Err("Item is not an image".into());
    }
    let b64 = item
        .content
        .strip_prefix("data:image/png;base64,")
        .or_else(|| item.content.strip_prefix("data:image/jpeg;base64,"))
        .unwrap_or(item.content.as_str())
        .to_string();
    let bytes = decode_png_bounded(&b64)?;
    let (width, height) = image_dimensions_from_png(&bytes).unwrap_or((0, 0));
    let capture = CaptureResult {
        png_base64: b64,
        width,
        height,
        saved_path: None,
    };
    open_snip_editor(app, state, Some(capture))
}

/// `$XDG_PICTURES_DIR/Screenshots` (usually `~/Pictures/Screenshots`).
fn screenshot_dir() -> Result<std::path::PathBuf, String> {
    let pictures = dirs::picture_dir().or_else(|| dirs::home_dir().map(|h| h.join("Pictures")));
    let pictures = pictures.ok_or_else(|| "Could not resolve Pictures directory".to_string())?;
    Ok(pictures.join("Screenshots"))
}

fn default_screenshot_name() -> String {
    format!(
        "Screenshot_{}.png",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    )
}

fn write_png_to_screenshots(bytes: &[u8], filename: &str) -> Result<String, String> {
    let folder = screenshot_dir()?;
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let name = sanitize_screenshot_filename(filename)?;
    let path = folder.join(name);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

fn image_dimensions_from_png(bytes: &[u8]) -> Result<(u32, u32), String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    Ok((img.width(), img.height()))
}

fn finalize_snip(
    app: &AppHandle,
    state: &State<'_, AppState>,
    mut result: CaptureResult,
) -> Result<CaptureResult, String> {
    let png_bytes = decode_png_bounded(&result.png_base64)?;
    let (content, preview) = image_content_and_preview(&png_bytes)?;
    write_item_to_clipboard(ClipItemType::Image, &content)?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let _ = db.insert_item(ClipItemType::Image, &content, &preview);
    }
    emit_history_changed(app);

    // Auto-save under $XDG_PICTURES_DIR/Screenshots; never fail the snip if this fails.
    match write_png_to_screenshots(&png_bytes, &default_screenshot_name()) {
        Ok(path) => {
            eprintln!("snip saved to {path}");
            result.saved_path = Some(path);
        }
        Err(err) => eprintln!("snip auto-save failed: {err}"),
    }

    if let Ok(mut last) = state.last_capture.lock() {
        *last = Some(result.clone());
    }

    app.emit("snip-captured", &result)
        .map_err(|e| e.to_string())?;
    windows::show_snip_toast(app)?;
    Ok(result)
}
