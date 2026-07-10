use crate::clipboard::{
    emit_history_changed,
    monitor::{image_content_and_preview, preview_text, write_item_to_clipboard, write_text},
    ClipItemSummary, ClipItemType,
};
use crate::db::Database;
use crate::focus_target;
use crate::settings::{self, AppSettings};
use crate::snip::{capture_fullscreen, capture_region, capture_window, list_windows, CaptureResult};
use crate::windows;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub focus_target: focus_target::FocusTargetStore,
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
    emit_history_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn clear_unpinned(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.clear_unpinned().map_err(|e| e.to_string())?;
    drop(db);
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
    write_text(&text)?;
    windows::hide_clipboard_panel(&app);
    focus_target::paste_after_hide(
        &state.focus_target,
        focus_target::PasteMode::TypeText,
        Some(&text),
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

    match item.item_type {
        ClipItemType::Text => {
            focus_target::paste_after_hide(
                &state.focus_target,
                focus_target::PasteMode::TypeText,
                Some(&item.content),
            );
        }
        ClipItemType::Image => {
            focus_target::paste_after_hide(
                &state.focus_target,
                focus_target::PasteMode::ClipboardPaste,
                None,
            );
        }
    }
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

#[tauri::command]
pub fn snip_fullscreen(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<CaptureResult, String> {
    let result = capture_fullscreen().map_err(|e| e.to_string())?;
    finalize_snip(&app, &state, &result)?;
    Ok(result)
}

#[tauri::command]
pub fn snip_window(
    app: AppHandle,
    state: State<'_, AppState>,
    window_id: u32,
) -> Result<CaptureResult, String> {
    let result = capture_window(window_id).map_err(|e| e.to_string())?;
    finalize_snip(&app, &state, &result)?;
    Ok(result)
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
    let result = capture_region(x, y, width, height).map_err(|e| e.to_string())?;
    finalize_snip(&app, &state, &result)?;
    Ok(result)
}

#[tauri::command]
pub fn copy_png_to_clipboard(
    app: AppHandle,
    state: State<'_, AppState>,
    png_base64: String,
) -> Result<(), String> {
    let bytes = STANDARD.decode(png_base64).map_err(|e| e.to_string())?;
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
    let bytes = STANDARD.decode(png_base64).map_err(|e| e.to_string())?;
    let pictures = dirs::picture_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let folder = pictures.join("ClipnPaste");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let name = filename.unwrap_or_else(|| {
        format!(
            "snip_{}.png",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        )
    });
    let path = folder.join(name);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

fn finalize_snip(
    app: &AppHandle,
    state: &State<'_, AppState>,
    result: &CaptureResult,
) -> Result<(), String> {
    let png_bytes = STANDARD
        .decode(&result.png_base64)
        .map_err(|e| e.to_string())?;
    let (content, preview) = image_content_and_preview(&png_bytes)?;
    write_item_to_clipboard(ClipItemType::Image, &content)?;
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let _ = db.insert_item(ClipItemType::Image, &content, &preview);
    }
    emit_history_changed(app);
    app.emit("snip-captured", result)
        .map_err(|e| e.to_string())?;
    windows::show_snip_toast(app)?;
    Ok(())
}
