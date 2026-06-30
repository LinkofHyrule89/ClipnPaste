use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

const CLIPBOARD_LABEL: &str = "clipboard";
const SNIP_TOOLBAR_LABEL: &str = "snip-toolbar";
const SNIP_OVERLAY_LABEL: &str = "snip-overlay";
const SNIP_TOAST_LABEL: &str = "snip-toast";
const SNIP_EDITOR_LABEL: &str = "snip-editor";

const CLIPBOARD_WIDTH: u32 = 420;
const CLIPBOARD_HEIGHT: u32 = 560;

pub fn show_clipboard_panel(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(CLIPBOARD_LABEL) {
        position_bottom_right(&window, CLIPBOARD_WIDTH, CLIPBOARD_HEIGHT)?;
        window
            .set_always_on_top(true)
            .map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        CLIPBOARD_LABEL,
        WebviewUrl::App("index.html?window=clipboard".into()),
    )
    .title("Clipboard")
    .inner_size(CLIPBOARD_WIDTH as f64, CLIPBOARD_HEIGHT as f64)
    .resizable(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    position_bottom_right(&window, CLIPBOARD_WIDTH, CLIPBOARD_HEIGHT)?;
    Ok(())
}

pub fn hide_clipboard_panel(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(CLIPBOARD_LABEL) {
        let _ = window.hide();
    }
}

pub fn show_snip_toolbar(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SNIP_TOOLBAR_LABEL) {
        center_top(&window)?;
        window
            .set_always_on_top(true)
            .map_err(|e| e.to_string())?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        SNIP_TOOLBAR_LABEL,
        WebviewUrl::App("index.html?window=snip-toolbar".into()),
    )
    .title("Snip")
    .inner_size(236.0, 44.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    center_top(&window)?;
    Ok(())
}

pub fn hide_snip_toolbar(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SNIP_TOOLBAR_LABEL) {
        let _ = window.hide();
    }
}

pub fn show_snip_overlay(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SNIP_OVERLAY_LABEL) {
        fullscreen(&window)?;
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        SNIP_OVERLAY_LABEL,
        WebviewUrl::App("index.html?window=snip-overlay".into()),
    )
    .title("Select region")
    .fullscreen(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    fullscreen(&window)?;
    Ok(())
}

pub fn hide_snip_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SNIP_OVERLAY_LABEL) {
        let _ = window.hide();
    }
}

pub fn show_snip_toast(app: &AppHandle) -> Result<(), String> {
    ensure_snip_toast(app)?;
    if let Some(window) = app.get_webview_window(SNIP_TOAST_LABEL) {
        position_bottom_right(&window, 280, 96)?;
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn ensure_snip_toast(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(SNIP_TOAST_LABEL).is_some() {
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        SNIP_TOAST_LABEL,
        WebviewUrl::App("index.html?window=snip-toast".into()),
    )
    .title("Snip captured")
    .inner_size(280.0, 96.0)
    .resizable(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn show_snip_editor(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(SNIP_EDITOR_LABEL) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        SNIP_EDITOR_LABEL,
        WebviewUrl::App("index.html?window=snip-editor".into()),
    )
    .title("Snip & Sketch")
    .inner_size(960.0, 720.0)
    .resizable(true)
    .decorations(true)
    .visible(true)
    .build()
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn center_top(window: &tauri::WebviewWindow) -> Result<(), String> {
    if let Ok(monitor) = window.current_monitor() {
        if let Some(monitor) = monitor {
            let size = monitor.size();
            let pos = monitor.position();
            let width = window.outer_size().map(|s| s.width).unwrap_or(320);
            let x = pos.x + ((size.width as i32 - width as i32) / 2);
            let y = pos.y + 48;
            window
                .set_position(PhysicalPosition::new(x, y))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn position_bottom_right(
    window: &tauri::WebviewWindow,
    default_width: u32,
    default_height: u32,
) -> Result<(), String> {
    if let Ok(monitor) = window.current_monitor() {
        if let Some(monitor) = monitor {
            let size = monitor.size();
            let pos = monitor.position();
            let win_size = window
                .outer_size()
                .unwrap_or(PhysicalSize::new(default_width, default_height));
            let x = pos.x + size.width as i32 - win_size.width as i32 - 24;
            let y = pos.y + size.height as i32 - win_size.height as i32 - 48;
            window
                .set_position(PhysicalPosition::new(x, y))
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn fullscreen(window: &tauri::WebviewWindow) -> Result<(), String> {
    window.set_fullscreen(true).map_err(|e| e.to_string())
}