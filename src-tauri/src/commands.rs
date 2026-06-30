use crate::clipboard::{
    monitor::write_item_to_clipboard, ClipItemSummary, ClipItemType,
};
use crate::db::Database;
use crate::snip::{capture_fullscreen, capture_region, capture_window, list_windows, CaptureResult};
use crate::windows;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Result<Vec<ClipItemSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_items()
        .map(|items| items.into_iter().map(ClipItemSummary::from).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pin_item(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_pinned(&id, true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn unpin_item(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_pinned(&id, false).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_item(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_item(&id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_unpinned(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.clear_unpinned().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn copy_item_to_clipboard(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let item = db.get_item(&id).map_err(|e| e.to_string())?.ok_or("Item not found")?;
    write_item_to_clipboard(item.item_type, &item.content)
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
pub fn copy_png_to_clipboard(png_base64: String) -> Result<(), String> {
    let bytes = STANDARD.decode(png_base64).map_err(|e| e.to_string())?;
    crate::clipboard::monitor::write_image_png(&bytes)
}

#[tauri::command]
pub fn save_png(png_base64: String, filename: Option<String>) -> Result<String, String> {
    let bytes = STANDARD.decode(png_base64).map_err(|e| e.to_string())?;
    let pictures = dirs::picture_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let folder = pictures.join("ClipnPaste");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let name = filename.unwrap_or_else(|| {
        format!("snip_{}.png", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
    });
    let path = folder.join(name);
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

fn finalize_snip(app: &AppHandle, state: &State<'_, AppState>, result: &CaptureResult) -> Result<(), String> {
    let content = format!("data:image/png;base64,{}", result.png_base64);
    write_item_to_clipboard(ClipItemType::Image, &content)?;
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let _ = db.insert_item(ClipItemType::Image, &content, &content);
    app.emit("snip-captured", result).map_err(|e| e.to_string())?;
    windows::show_snip_toast(app)?;
    Ok(())
}