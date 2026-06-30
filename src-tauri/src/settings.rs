use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_true")]
    pub emoji_tab_enabled: bool,
    #[serde(default = "default_true")]
    pub gif_tab_enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            emoji_tab_enabled: true,
            gif_tab_enabled: true,
        }
    }
}

pub fn settings_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("clipnpaste")
        .join("settings.json")
}

pub fn load() -> AppSettings {
    let path = settings_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return AppSettings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn emoji_enabled(settings: &Arc<Mutex<AppSettings>>) -> bool {
    settings
        .lock()
        .map(|current| current.emoji_tab_enabled)
        .unwrap_or(true)
}

pub fn get_locked(settings: &Arc<Mutex<AppSettings>>) -> AppSettings {
    settings
        .lock()
        .map(|current| current.clone())
        .unwrap_or_default()
}

pub fn apply_settings(
    app: &AppHandle,
    settings_store: &Arc<Mutex<AppSettings>>,
    settings: AppSettings,
) -> Result<(), String> {
    save(&settings)?;
    {
        let mut current = settings_store.lock().map_err(|e| e.to_string())?;
        *current = settings.clone();
    }
    app.emit("settings-changed", settings)
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn open_keyboard_shortcuts() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if is_cinnamon() {
            return spawn_settings("cinnamon-settings", &["keyboard", "--tab", "shortcuts"]);
        }
        if command_exists("gnome-control-center") {
            return spawn_settings("gnome-control-center", &["keyboard"]);
        }
        return Err(
            "Keyboard shortcut settings are only available on Cinnamon or GNOME desktops."
                .to_string(),
        );
    }

    #[cfg(not(target_os = "linux"))]
    {
        Err("Keyboard shortcut settings are only available on Linux.".to_string())
    }
}

#[cfg(target_os = "linux")]
fn is_cinnamon() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|desktop| desktop.to_ascii_lowercase().contains("cinnamon"))
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn spawn_settings(command: &str, args: &[&str]) -> Result<(), String> {
    Command::new(command)
        .args(args)
        .spawn()
        .map_err(|e| format!("failed to open keyboard settings: {e}"))?;
    Ok(())
}

pub fn init_settings() -> Arc<Mutex<AppSettings>> {
    Arc::new(Mutex::new(load()))
}