use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::AppHandle;
use x11rb::connection::Connection;
use x11rb::properties::WmClass;
use x11rb::protocol::xproto::{
    ConnectionExt, GrabMode, KeyButMask, ModMask, Window, KEY_PRESS_EVENT, KEY_RELEASE_EVENT,
};
use x11rb::protocol::xtest::ConnectionExt as XTestConnectionExt;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use xkeysym::RawKeysym;

use crate::windows;

static CHORD_USED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HotkeyAction {
    Clipboard,
    Snip,
}

struct HotkeyBinding {
    action: HotkeyAction,
    keycode: u8,
    mods: ModMask,
    /// Plain grabs (modifier 0) check physical modifier state and replay otherwise.
    plain: bool,
}

pub fn setup(app: &AppHandle) -> Result<(), String> {
    let app_handle = app.clone();
    std::thread::Builder::new()
        .name("clipnpaste-hotkeys".into())
        .spawn(move || {
            if let Err(err) = run_hotkey_loop(app_handle) {
                eprintln!("ClipnPaste hotkey thread exited: {err}");
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn chord_mod_mask() -> KeyButMask {
    KeyButMask::CONTROL | KeyButMask::SHIFT | KeyButMask::MOD4 | KeyButMask::MOD1
}

fn run_hotkey_loop(app: AppHandle) -> Result<(), String> {
    let (conn, screen) = RustConnection::connect(None)
        .map_err(|e| format!("failed to connect to X11 display: {e}"))?;

    conn.xtest_get_version(2, 2)
        .map_err(|e| format!("failed to initialize XTest extension: {e}"))?
        .reply()
        .map_err(|e| format!("failed to initialize XTest extension: {e}"))?;

    let root = conn.setup().roots[screen].root;
    let super_keycodes = keycodes_for_keysyms(&conn, &[xkeysym::key::Super_L, xkeysym::key::Super_R])?;
    let shift_keycodes = keycodes_for_keysyms(&conn, &[xkeysym::key::Shift_L, xkeysym::key::Shift_R])?;
    let escape_keycode = keycodes_for_keysyms(&conn, &[xkeysym::key::Escape])?
        .into_iter()
        .next()
        .ok_or_else(|| "unable to resolve Escape keycode".to_string())?;

    let v_keycode = required_keycode(&conn, xkeysym::key::V, "V")?;
    let s_keycode = required_keycode(&conn, xkeysym::key::S, "S")?;

    let bindings = [
        HotkeyBinding {
            action: HotkeyAction::Clipboard,
            keycode: v_keycode,
            mods: ModMask::from(KeyButMask::MOD4.bits()),
            plain: false,
        },
        HotkeyBinding {
            action: HotkeyAction::Clipboard,
            keycode: v_keycode,
            mods: ModMask::default(),
            plain: true,
        },
        HotkeyBinding {
            action: HotkeyAction::Snip,
            keycode: s_keycode,
            mods: ModMask::from(KeyButMask::MOD4.bits() | KeyButMask::SHIFT.bits()),
            plain: false,
        },
        HotkeyBinding {
            action: HotkeyAction::Snip,
            keycode: s_keycode,
            mods: ModMask::default(),
            plain: true,
        },
    ];

    let mut registered = BTreeMap::<u8, Vec<HotkeyBinding>>::new();
    for binding in bindings {
        register_binding(&conn, root, &mut registered, binding)?;
    }

    let mut super_was_down = super_keys_down(&conn, &super_keycodes)?;
    let mut last_chord_at: Option<Instant> = None;

    loop {
        if let Some(event) = conn.poll_for_event().map_err(|e| e.to_string())? {
            if let Event::KeyPress(event) = event {
                let keycode = event.detail;
                let event_mods = ModMask::from((event.state & chord_mod_mask()).bits());

                if let Some(entries) = registered.get(&keycode) {
                    for binding in entries {
                        if event_mods != binding.mods {
                            continue;
                        }

                        if binding.plain {
                            if handle_plain_press(
                                binding,
                                &conn,
                                root,
                                &super_keycodes,
                                &shift_keycodes,
                                &app,
                            )? {
                                CHORD_USED.store(true, Ordering::SeqCst);
                                last_chord_at = Some(Instant::now());
                            }
                        } else {
                            CHORD_USED.store(true, Ordering::SeqCst);
                            last_chord_at = Some(Instant::now());
                            dispatch_action(&app, binding.action);
                        }
                        break;
                    }
                }
            }
        }

        let super_is_down = super_keys_down(&conn, &super_keycodes)?;
        if super_was_down
            && !super_is_down
            && CHORD_USED.swap(false, Ordering::SeqCst)
            && last_chord_at.is_some_and(|t| t.elapsed() < Duration::from_millis(800))
        {
            dismiss_mintmenu(escape_keycode);
        }
        super_was_down = super_is_down;

        std::thread::sleep(Duration::from_millis(1));
    }
}

fn handle_plain_press(
    binding: &HotkeyBinding,
    conn: &RustConnection,
    root: Window,
    super_keycodes: &[u8],
    shift_keycodes: &[u8],
    app: &AppHandle,
) -> Result<bool, String> {
    let should_trigger = match binding.action {
        HotkeyAction::Clipboard => super_keys_down(conn, super_keycodes)?,
        HotkeyAction::Snip => {
            super_keys_down(conn, super_keycodes)? && shift_keys_down(conn, shift_keycodes)?
        }
    };

    if should_trigger {
        dispatch_action(app, binding.action);
        Ok(true)
    } else {
        replay_key(conn, root, binding.keycode)?;
        Ok(false)
    }
}

fn replay_key(conn: &RustConnection, root: Window, keycode: u8) -> Result<(), String> {
    conn.xtest_fake_input(KEY_PRESS_EVENT, keycode, 0, root, 0, 0, 0)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| format!("failed to replay key press: {e:?}"))?;
    conn.xtest_fake_input(KEY_RELEASE_EVENT, keycode, 0, root, 0, 0, 0)
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| format!("failed to replay key release: {e:?}"))?;
    conn.flush().map_err(|e| e.to_string())?;
    Ok(())
}

fn dispatch_action(app: &AppHandle, action: HotkeyAction) {
    let app_handle = app.clone();
    let _ = app.clone().run_on_main_thread(move || match action {
        HotkeyAction::Clipboard => {
            let _ = windows::show_clipboard_panel(&app_handle);
        }
        HotkeyAction::Snip => {
            let _ = windows::show_snip_toolbar(&app_handle);
        }
    });
}

fn ignored_mods() -> [ModMask; 4] {
    [
        ModMask::default(),
        ModMask::M2,
        ModMask::LOCK,
        ModMask::M2 | ModMask::LOCK,
    ]
}

fn register_binding(
    conn: &RustConnection,
    root: Window,
    registered: &mut BTreeMap<u8, Vec<HotkeyBinding>>,
    binding: HotkeyBinding,
) -> Result<(), String> {
    for lock_mod in ignored_mods() {
        conn.grab_key(
            false,
            root,
            binding.mods | lock_mod,
            binding.keycode,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )
        .map_err(|e| format!("failed to register {:?} hotkey: {e}", binding.action))?
        .check()
        .map_err(|e| format!("failed to register {:?} hotkey: {e:?}", binding.action))?;
    }

    registered.entry(binding.keycode).or_default().push(binding);
    Ok(())
}

fn required_keycode(conn: &RustConnection, keysym: RawKeysym, label: &str) -> Result<u8, String> {
    keycodes_for_keysyms(conn, &[keysym])?
        .into_iter()
        .next()
        .ok_or_else(|| format!("unable to resolve {label} keycode"))
}

fn keycodes_for_keysyms(conn: &RustConnection, keysyms: &[RawKeysym]) -> Result<Vec<u8>, String> {
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
    let mut keycodes = Vec::new();

    for (index, chunk) in mapping.keysyms.chunks(keysyms_per_keycode).enumerate() {
        if chunk.iter().any(|sym| keysyms.contains(sym)) {
            keycodes.push(min_keycode + index as u8);
        }
    }

    Ok(keycodes)
}

fn key_down(conn: &RustConnection, keycode: u8) -> Result<bool, String> {
    let keymap = conn
        .query_keymap()
        .map_err(|e| e.to_string())?
        .reply()
        .map_err(|e| e.to_string())?;

    let byte = keycode / 8;
    let bit = keycode % 8;
    Ok(keymap.keys[byte as usize] & (1 << bit) != 0)
}

fn super_keys_down(conn: &RustConnection, keycodes: &[u8]) -> Result<bool, String> {
    keycodes
        .iter()
        .try_fold(false, |acc, &keycode| key_down(conn, keycode).map(|down| acc || down))
}

fn shift_keys_down(conn: &RustConnection, keycodes: &[u8]) -> Result<bool, String> {
    keycodes
        .iter()
        .try_fold(false, |acc, &keycode| key_down(conn, keycode).map(|down| acc || down))
}

fn dismiss_mintmenu(escape_keycode: u8) {
    std::thread::sleep(Duration::from_millis(60));

    let Ok((conn, screen)) = RustConnection::connect(None) else {
        return;
    };

    let root = conn.setup().roots[screen].root;
    let Some(menu_window) = find_mintmenu_window(&conn, root) else {
        return;
    };

    let Ok(attrs) = conn
        .get_window_attributes(menu_window)
        .map_err(|e| e.to_string())
        .and_then(|cookie| cookie.reply().map_err(|e| e.to_string()))
    else {
        return;
    };

    if attrs.map_state != x11rb::protocol::xproto::MapState::VIEWABLE {
        return;
    }

    let _ = send_escape_to_window(&conn, root, menu_window, escape_keycode);
}

fn find_mintmenu_window(conn: &RustConnection, window: Window) -> Option<Window> {
    let Ok(tree) = conn
        .query_tree(window)
        .map_err(|e| e.to_string())
        .and_then(|cookie| cookie.reply().map_err(|e| e.to_string()))
    else {
        return None;
    };

    for child in tree.children {
        if window_matches_mintmenu(conn, child) {
            return Some(child);
        }
        if let Some(found) = find_mintmenu_window(conn, child) {
            return Some(found);
        }
    }

    None
}

fn window_matches_mintmenu(conn: &RustConnection, window: Window) -> bool {
    let Ok(cookie) = WmClass::get(conn, window) else {
        return false;
    };

    let Ok(Some(wm_class)) = cookie.reply_unchecked() else {
        return false;
    };

    let instance = String::from_utf8_lossy(wm_class.instance()).to_lowercase();
    let class = String::from_utf8_lossy(wm_class.class()).to_lowercase();
    instance.contains("mintmenu") || class.contains("mintmenu")
}

fn send_escape_to_window(
    conn: &RustConnection,
    root: Window,
    window: Window,
    escape_keycode: u8,
) -> Result<(), String> {
    use x11rb::protocol::xproto::{KeyPressEvent, KeyReleaseEvent};

    let press = KeyPressEvent {
        response_type: KEY_PRESS_EVENT,
        detail: escape_keycode,
        sequence: 0,
        time: 0,
        root,
        event: window,
        child: 0,
        root_x: 0,
        root_y: 0,
        event_x: 0,
        event_y: 0,
        state: KeyButMask::from(0u16),
        same_screen: true,
    };

    let release = KeyReleaseEvent {
        response_type: KEY_RELEASE_EVENT,
        detail: escape_keycode,
        sequence: 0,
        time: 1,
        root,
        event: window,
        child: 0,
        root_x: 0,
        root_y: 0,
        event_x: 0,
        event_y: 0,
        state: KeyButMask::from(0u16),
        same_screen: true,
    };

    conn.send_event(false, window, x11rb::protocol::xproto::EventMask::KEY_PRESS, press)
        .map_err(|e| e.to_string())?;
    conn.send_event(false, window, x11rb::protocol::xproto::EventMask::KEY_RELEASE, release)
        .map_err(|e| e.to_string())?;
    conn.flush().map_err(|e| e.to_string())?;

    Ok(())
}