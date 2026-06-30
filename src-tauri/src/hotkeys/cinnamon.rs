use std::path::PathBuf;
use std::process::Command;

const CLIPBOARD_ID: &str = "custom3";
const SNIP_ID: &str = "custom4";

pub fn register() -> Result<bool, String> {
    if !is_cinnamon() {
        return Ok(false);
    }

    let cli = resolve_cli_path()?;
    register_binding(
        CLIPBOARD_ID,
        "ClipnPaste Clipboard",
        "['<Super>v']",
        &format!("{} clipboard", cli.display()),
    )?;
    register_binding(
        SNIP_ID,
        "ClipnPaste Snip",
        "['<Super><Shift>s']",
        &format!("{} snip", cli.display()),
    )?;

    Ok(true)
}

fn is_cinnamon() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|desktop| desktop.to_ascii_lowercase().contains("cinnamon"))
        .unwrap_or(false)
        && Command::new("gsettings")
            .args(["list-schemas"])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .contains("org.cinnamon.desktop.keybindings")
            })
            .unwrap_or(false)
}

fn resolve_cli_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("CLIPNPASTE_CLI") {
        return Ok(PathBuf::from(path));
    }

    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name("clipnpaste-cli");
        if sibling.is_file() {
            return Ok(sibling);
        }
    }

    let which = Command::new("which")
        .arg("clipnpaste-cli")
        .output()
        .map_err(|e| e.to_string())?;
    if which.status.success() {
        let path = String::from_utf8_lossy(&which.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    Err("clipnpaste-cli not found; reinstall ClipnPaste".to_string())
}

fn register_binding(id: &str, name: &str, binding: &str, command: &str) -> Result<(), String> {
    ensure_custom_id(id)?;
    gsettings_set(&format!("org.cinnamon.desktop.keybindings.custom-keybinding:/{id}/"), "name", name)?;
    gsettings_set(
        &format!("org.cinnamon.desktop.keybindings.custom-keybinding:/{id}/"),
        "binding",
        binding,
    )?;
    gsettings_set(
        &format!("org.cinnamon.desktop.keybindings.custom-keybinding:/{id}/"),
        "command",
        command,
    )?;
    Ok(())
}

fn ensure_custom_id(id: &str) -> Result<(), String> {
    let current = gsettings_get("org.cinnamon.desktop.keybindings", "custom-list")?;
    let mut entries = parse_gsettings_list(&current);
    if !entries.iter().any(|entry| entry == id) {
        entries.push(id.to_string());
        let serialized = format_gsettings_list(&entries);
        gsettings_set("org.cinnamon.desktop.keybindings", "custom-list", &serialized)?;
    }
    Ok(())
}

fn gsettings_get(schema: &str, key: &str) -> Result<String, String> {
    let output = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn gsettings_set(schema: &str, key: &str, value: &str) -> Result<(), String> {
    let output = Command::new("gsettings")
        .args(["set", schema, key, value])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

fn parse_gsettings_list(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|entry| entry.trim().trim_matches('\'').to_string())
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn format_gsettings_list(values: &[String]) -> String {
    let inner = values
        .iter()
        .map(|value| format!("'{value}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{inner}]")
}