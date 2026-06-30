#[cfg(target_os = "linux")]
mod cinnamon;
#[cfg(target_os = "linux")]
mod x11;

use tauri::AppHandle;

pub fn setup(app: &AppHandle) -> Result<(), String> {
    crate::ipc::start(app.clone())?;

    #[cfg(target_os = "linux")]
    {
        if cinnamon::register()? {
            x11::start_menu_guard();
            return Ok(());
        }
        return x11::setup(app);
    }

    #[cfg(not(target_os = "linux"))]
    {
        setup_tauri_shortcuts(app)
    }
}

#[cfg(not(target_os = "linux"))]
fn setup_tauri_shortcuts(app: &AppHandle) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut("Super+V", move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = crate::windows::show_clipboard_panel(&app_handle);
            }
        })
        .map_err(|e| e.to_string())?;

    let app_handle = app.clone();
    app.global_shortcut()
        .on_shortcut("Super+Shift+S", move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = crate::windows::show_snip_toolbar(&app_handle);
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}