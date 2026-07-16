use std::fs;
use std::io::Read;
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};

use crate::commands::AppState;
use crate::focus_target;
use crate::settings;
use crate::windows::{self, ClipboardTab};

pub static CHORD_USED: AtomicBool = AtomicBool::new(false);
static LAST_CHORD_MS: AtomicU64 = AtomicU64::new(0);

/// Known IPC commands accepted from `clipnpaste-cli` / local socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcCommand {
    Clipboard,
    Emoji,
    Snip,
    Settings,
}

/// Parse a raw socket payload into an allowlisted command.
pub fn parse_ipc_command(raw: &str) -> Option<IpcCommand> {
    match raw.trim() {
        "clipboard" => Some(IpcCommand::Clipboard),
        "emoji" => Some(IpcCommand::Emoji),
        "snip" => Some(IpcCommand::Snip),
        "settings" => Some(IpcCommand::Settings),
        _ => None,
    }
}

pub fn socket_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("clipnpaste")
        .join("ipc.sock")
}

pub fn start(app: AppHandle) -> Result<(), String> {
    let socket_path = socket_path();
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }
    let _ = fs::remove_file(&socket_path);

    let app = Arc::new(app);
    std::thread::Builder::new()
        .name("clipnpaste-ipc".into())
        .spawn(move || {
            let listener = match UnixListener::bind(&socket_path) {
                Ok(listener) => listener,
                Err(err) => {
                    eprintln!("ClipnPaste IPC bind failed: {err}");
                    return;
                }
            };

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Err(err) =
                    fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600))
                {
                    eprintln!("ClipnPaste IPC chmod failed: {err}");
                }
            }

            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut buf = [0u8; 32];
                let Ok(n) = stream.read(&mut buf) else { continue };
                let cmd = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                mark_chord_used();
                dispatch(&app, &cmd);
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn mark_chord_used() {
    CHORD_USED.store(true, Ordering::SeqCst);
    LAST_CHORD_MS.store(now_ms(), Ordering::SeqCst);
}

pub fn chord_used_recently(within_ms: u64) -> bool {
    let elapsed = now_ms().saturating_sub(LAST_CHORD_MS.load(Ordering::SeqCst));
    elapsed < within_ms
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn dispatch(app: &AppHandle, cmd: &str) {
    let Some(cmd) = parse_ipc_command(cmd) else {
        return;
    };

    if matches!(cmd, IpcCommand::Emoji | IpcCommand::Clipboard) {
        let state = app.state::<AppState>();
        focus_target::load_into_store(&state.focus_target);
    }

    let app_handle = app.clone();
    let _ = app.clone().run_on_main_thread(move || match cmd {
        IpcCommand::Clipboard => {
            let _ = windows::show_clipboard_panel(&app_handle, ClipboardTab::History);
        }
        IpcCommand::Emoji => {
            let state = app_handle.state::<AppState>();
            if settings::emoji_enabled(&state.settings) {
                let _ = windows::show_clipboard_panel(&app_handle, ClipboardTab::Emoji);
            }
        }
        IpcCommand::Snip => {
            let _ = windows::show_snip_toolbar(&app_handle);
        }
        IpcCommand::Settings => {
            let _ = windows::show_settings_window(&app_handle);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_known_commands() {
        assert_eq!(parse_ipc_command("clipboard"), Some(IpcCommand::Clipboard));
        assert_eq!(parse_ipc_command("emoji"), Some(IpcCommand::Emoji));
        assert_eq!(parse_ipc_command("snip"), Some(IpcCommand::Snip));
        assert_eq!(parse_ipc_command("settings"), Some(IpcCommand::Settings));
    }

    #[test]
    fn parse_trims_whitespace() {
        assert_eq!(
            parse_ipc_command("  clipboard\n"),
            Some(IpcCommand::Clipboard)
        );
    }

    #[test]
    fn parse_rejects_unknown() {
        assert_eq!(parse_ipc_command(""), None);
        assert_eq!(parse_ipc_command("quit"), None);
        assert_eq!(parse_ipc_command("clipboard; rm -rf /"), None);
        assert_eq!(parse_ipc_command("CLIPBOARD"), None);
    }
}
