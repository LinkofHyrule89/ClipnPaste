mod clipboard;
mod commands;
mod db;
mod hotkeys;
mod ipc;
mod snip;
mod windows;

use commands::AppState;
use clipboard::ClipboardMonitor;
use db::Database;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(not(target_os = "linux"))]
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build());

    #[cfg(target_os = "linux")]
    let builder = tauri::Builder::default();

    builder
        .setup(|app| {
            let database = Database::open().expect("failed to open database");
            let db = Arc::new(Mutex::new(database));
            let _monitor = ClipboardMonitor::start(db.clone());

            app.manage(AppState { db });

            setup_tray(app.handle())?;
            hotkeys::setup(app.handle()).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            preload_windows(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_history,
            commands::pin_item,
            commands::unpin_item,
            commands::delete_item,
            commands::clear_unpinned,
            commands::copy_item_to_clipboard,
            commands::list_capture_windows,
            commands::snip_fullscreen,
            commands::snip_window,
            commands::snip_region,
            commands::copy_png_to_clipboard,
            commands::save_png,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_clipboard = MenuItem::with_id(app, "open_clipboard", "Open Clipboard", true, None::<&str>)?;
    let open_snip = MenuItem::with_id(app, "open_snip", "Snipping Tool", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_clipboard, &open_snip, &quit])?;

    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open_clipboard" => {
                let _ = windows::show_clipboard_panel(app);
            }
            "open_snip" => {
                let _ = windows::show_snip_toolbar(app);
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let _ = windows::show_clipboard_panel(&app);
            }
        })
        .build(app)?;

    Ok(())
}

fn preload_windows(app: &AppHandle) -> Result<(), String> {
    let _ = windows::show_snip_overlay(app);
    windows::hide_snip_overlay(app);
    let _ = windows::show_snip_toast(app);
    if let Some(window) = app.get_webview_window("snip-toast") {
        let _ = window.hide();
    }
    Ok(())
}

