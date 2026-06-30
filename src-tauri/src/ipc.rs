use std::fs;
use std::io::Read;
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::AppHandle;

use crate::windows;

pub static CHORD_USED: AtomicBool = AtomicBool::new(false);
static LAST_CHORD_MS: AtomicU64 = AtomicU64::new(0);

pub fn socket_path() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("clipnpaste")
        .join("ipc.sock")
}

pub fn start(app: AppHandle) -> Result<(), String> {
    let socket_path = socket_path();
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
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
    let app_handle = app.clone();
    let cmd = cmd.to_string();
    let _ = app.clone().run_on_main_thread(move || match cmd.as_str() {
        "clipboard" => {
            let _ = windows::show_clipboard_panel(&app_handle);
        }
        "snip" => {
            let _ = windows::show_snip_toolbar(&app_handle);
        }
        _ => {}
    });
}