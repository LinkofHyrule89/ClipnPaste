use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(target_os = "linux")]
use x11rb::connection::Connection;
#[cfg(target_os = "linux")]
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt, InputFocus, KeyButMask, KeyPressEvent, KeyReleaseEvent, Window,
    KEY_PRESS_EVENT, KEY_RELEASE_EVENT,
};
#[cfg(target_os = "linux")]
use x11rb::rust_connection::RustConnection;
#[cfg(target_os = "linux")]
use xkeysym::RawKeysym;

pub type FocusTargetStore = Arc<Mutex<Option<u32>>>;

#[derive(Clone, Copy, Debug)]
pub enum PasteMode {
    TypeText,
    ClipboardPaste,
}

pub fn new_store() -> FocusTargetStore {
    Arc::new(Mutex::new(None))
}

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("clipnpaste")
}

pub fn focus_target_file() -> PathBuf {
    data_dir().join("focus_target")
}

pub fn capture_to_file() {
    #[cfg(target_os = "linux")]
    {
        let Some(id) = get_active_window_id().ok().filter(|id| is_pasteable_window(*id)) else {
            return;
        };
        let path = focus_target_file();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, id.to_string());
    }
}

pub fn load_into_store(store: &FocusTargetStore) {
    let from_file = fs::read_to_string(focus_target_file())
        .ok()
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .filter(|id| is_pasteable_window(*id));

    if let Some(id) = from_file {
        if let Ok(mut target) = store.lock() {
            *target = Some(id);
        }
    }
}

pub fn capture(store: &FocusTargetStore) {
    #[cfg(target_os = "linux")]
    {
        let Some(id) = get_active_window_id().ok().filter(|id| is_pasteable_window(*id)) else {
            return;
        };
        if let Ok(mut target) = store.lock() {
            *target = Some(id);
        }
        let _ = fs::write(focus_target_file(), id.to_string());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = store;
    }
}

pub fn paste_after_hide(store: &FocusTargetStore, mode: PasteMode, text: Option<&str>) {
    load_into_store(store);
    let Some(window_id) = store.lock().ok().and_then(|target| *target) else {
        eprintln!("ClipnPaste paste skipped: no focus target captured");
        return;
    };

    let owned_text = text.map(str::to_string);
    std::thread::Builder::new()
        .name("clipnpaste-paste".into())
        .spawn(move || {
            std::thread::sleep(Duration::from_millis(220));
            let result = match mode {
                PasteMode::TypeText => {
                    let Some(content) = owned_text.as_deref() else {
                        return;
                    };
                    send_type_to_window(window_id, content)
                }
                PasteMode::ClipboardPaste => send_clipboard_paste_to_window(window_id),
            };
            if let Err(err) = result {
                eprintln!("ClipnPaste paste failed for window {window_id}: {err}");
            }
        })
        .ok();
}

#[cfg(target_os = "linux")]
pub fn get_active_window_id() -> Result<u32, String> {
    if let Ok(id) = active_window_via_xdotool() {
        return Ok(id);
    }
    active_window_via_net_active()
}

#[cfg(not(target_os = "linux"))]
pub fn get_active_window_id() -> Result<u32, String> {
    Err("unsupported platform".to_string())
}

#[cfg(target_os = "linux")]
fn active_window_via_xdotool() -> Result<u32, String> {
    let output = xdotool_output(&["getactivewindow"])?;
    let id = output
        .trim()
        .parse::<u32>()
        .map_err(|_| "invalid xdotool window id".to_string())?;
    if id == 0 {
        return Err("no active window".to_string());
    }
    Ok(id)
}

#[cfg(target_os = "linux")]
fn active_window_via_net_active() -> Result<u32, String> {
    let (conn, screen) = RustConnection::connect(None)
        .map_err(|e| format!("failed to connect to X11 display: {e}"))?;
    let root = conn.setup().roots[screen].root;
    let atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?
        .atom;
    let reply = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
    let window_id = reply.value32().and_then(|mut values| values.next()).unwrap_or(0);
    if window_id == 0 {
        return Err("no active window".to_string());
    }
    Ok(window_id)
}

#[cfg(target_os = "linux")]
fn window_class(window_id: u32) -> Option<String> {
    let output = xdotool_output(&["getwindowclassname", &window_id.to_string()]).ok()?;
    let class = output.trim().to_ascii_lowercase();
    if class.is_empty() {
        None
    } else {
        Some(class)
    }
}

#[cfg(target_os = "linux")]
fn is_pasteable_window(window_id: u32) -> bool {
    let Some(class) = window_class(window_id) else {
        return true;
    };
    if class.contains("clipnpaste") {
        return false;
    }
    if class.contains("mintmenu") || (class.contains("cinnamon") && class.contains("menu")) {
        return false;
    }
    if class.contains("csd-") {
        return false;
    }
    true
}

#[cfg(not(target_os = "linux"))]
fn is_pasteable_window(_window_id: u32) -> bool {
    false
}

#[cfg(target_os = "linux")]
fn send_type_to_window(window_id: u32, text: &str) -> Result<(), String> {
    if command_exists("xdotool") {
        return send_type_via_xdotool(window_id, text);
    }
    send_clipboard_paste_to_window(window_id)
}

#[cfg(target_os = "linux")]
fn send_clipboard_paste_to_window(window_id: u32) -> Result<(), String> {
    if command_exists("xdotool") {
        return send_clipboard_paste_via_xdotool(window_id);
    }
    send_clipboard_paste_via_x11(window_id)
}

#[cfg(target_os = "linux")]
fn send_type_via_xdotool(window_id: u32, text: &str) -> Result<(), String> {
    focus_window(window_id)?;
    xdotool_status(&["type", "--delay", "1", "--", text])
}

#[cfg(target_os = "linux")]
fn send_clipboard_paste_via_xdotool(window_id: u32) -> Result<(), String> {
    focus_window(window_id)?;
    xdotool_status(&["key", "--clearmodifiers", "ctrl+v"])
}

#[cfg(target_os = "linux")]
fn focus_window(window_id: u32) -> Result<(), String> {
    xdotool_status(&["windowactivate", "--sync", &window_id.to_string()])?;
    std::thread::sleep(Duration::from_millis(80));
    Ok(())
}

#[cfg(target_os = "linux")]
fn xdotool_output(args: &[&str]) -> Result<String, String> {
    let output = Command::new("xdotool")
        .args(args)
        .output()
        .map_err(|e| format!("xdotool unavailable: {e}"))?;
    if !output.status.success() {
        return Err(format!("xdotool {} failed", args.first().unwrap_or(&"")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(target_os = "linux")]
fn xdotool_status(args: &[&str]) -> Result<(), String> {
    let status = Command::new("xdotool")
        .args(args)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("xdotool {} failed", args.first().unwrap_or(&"")));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn send_clipboard_paste_via_x11(window_id: u32) -> Result<(), String> {
    let (conn, screen) = RustConnection::connect(None).map_err(|e| e.to_string())?;
    let root = conn.setup().roots[screen].root;
    let window = window_id as Window;

    conn.set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| format!("failed to focus target window: {e:?}"))?;
    conn.flush().map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_millis(80));

    let ctrl_keycode = keycode_for_keysym(&conn, xkeysym::key::Control_L)
        .or_else(|_| keycode_for_keysym(&conn, xkeysym::key::Control_R))?;
    let v_keycode = keycode_for_keysym(&conn, xkeysym::key::V)?;
    let ctrl_mask = KeyButMask::CONTROL;

    send_key(&conn, root, window, ctrl_keycode, KeyButMask::from(0u16), true)?;
    send_key(&conn, root, window, v_keycode, ctrl_mask, true)?;
    send_key(&conn, root, window, v_keycode, ctrl_mask, false)?;
    send_key(
        &conn,
        root,
        window,
        ctrl_keycode,
        KeyButMask::from(0u16),
        false,
    )?;
    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn command_exists(command: &str) -> bool {
    Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn keycode_for_keysym(conn: &RustConnection, keysym: RawKeysym) -> Result<u8, String> {
    let setup = conn.setup();
    let min_keycode = setup.min_keycode;
    let max_keycode = setup.max_keycode;
    let count = max_keycode - min_keycode + 1;
    let mapping = conn
        .get_keyboard_mapping(min_keycode, count)
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;
    let keysyms_per_keycode = mapping.keysyms_per_keycode as usize;

    for (index, chunk) in mapping.keysyms.chunks(keysyms_per_keycode).enumerate() {
        if chunk.iter().any(|sym| *sym == keysym) {
            return Ok(min_keycode + index as u8);
        }
    }

    Err("unable to resolve keycode".to_string())
}

#[cfg(target_os = "linux")]
fn send_key(
    conn: &RustConnection,
    root: Window,
    window: Window,
    keycode: u8,
    state: KeyButMask,
    press: bool,
) -> Result<(), String> {
    if press {
        let event = KeyPressEvent {
            response_type: KEY_PRESS_EVENT,
            detail: keycode,
            sequence: 0,
            time: 0,
            root,
            event: window,
            child: 0,
            root_x: 0,
            root_y: 0,
            event_x: 0,
            event_y: 0,
            state,
            same_screen: true,
        };
        conn.send_event(false, window, x11rb::protocol::xproto::EventMask::KEY_PRESS, event)
            .map_err(|e| e.to_string())?;
    } else {
        let event = KeyReleaseEvent {
            response_type: KEY_RELEASE_EVENT,
            detail: keycode,
            sequence: 0,
            time: 1,
            root,
            event: window,
            child: 0,
            root_x: 0,
            root_y: 0,
            event_x: 0,
            event_y: 0,
            state,
            same_screen: true,
        };
        conn.send_event(false, window, x11rb::protocol::xproto::EventMask::KEY_RELEASE, event)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}