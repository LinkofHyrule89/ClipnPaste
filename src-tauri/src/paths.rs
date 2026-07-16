//! Safe filesystem helpers for screenshot paths and bounded image decode.

use crate::db::MAX_IMAGE_BYTES;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::path::{Component, Path, PathBuf};

/// Accept only a simple screenshot basename (no directories, no `..`).
pub fn sanitize_screenshot_filename(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Filename is empty".into());
    }
    if name.contains('\0') {
        return Err("Filename contains null byte".into());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("Filename must not contain path separators".into());
    }
    if name == "." || name == ".." {
        return Err("Invalid filename".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
    {
        return Err("Filename has invalid characters".into());
    }
    if !name.to_ascii_lowercase().ends_with(".png") {
        return Err("Filename must end with .png".into());
    }
    // Reject names that are only dots before extension or start with '.'
    let stem = &name[..name.len() - 4];
    if stem.is_empty() || stem.chars().all(|c| c == '.') {
        return Err("Invalid filename".into());
    }
    Ok(name.to_string())
}

/// True if `path` is exactly `base` or a descendant (component-wise, no `..`).
pub fn is_under_dir(path: &Path, base: &Path) -> bool {
    let path = normalize_lexical(path);
    let base = normalize_lexical(base);
    path.starts_with(&base)
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            Component::Normal(s) => out.push(s),
        }
    }
    out
}

/// Resolve where an edited/auto snip may be written.
///
/// - `None` / empty → `screenshots_dir / sanitized(default_name)`
/// - `Some(path)` → only the **basename** is used (must sanitize); always under
///   `screenshots_dir`. If the client supplied a full path, it must lexically
///   resolve under `screenshots_dir` or the call fails (no silent rewrite to
///   a different directory).
pub fn resolve_snip_save_path(
    screenshots_dir: &Path,
    path: Option<&str>,
    default_name: &str,
) -> Result<PathBuf, String> {
    let default_name = sanitize_screenshot_filename(default_name)?;
    let default_path = screenshots_dir.join(&default_name);

    let Some(raw) = path.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(default_path);
    };

    let candidate = PathBuf::from(raw);
    let file_name = candidate
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid save path".to_string())?;
    let safe_name = sanitize_screenshot_filename(file_name)?;

    // Single-component (basename only) is always OK under Screenshots.
    if candidate.components().count() == 1 {
        return Ok(screenshots_dir.join(safe_name));
    }

    // Multi-component or absolute: must stay under screenshots_dir.
    let full = if candidate.is_absolute() {
        candidate
    } else {
        screenshots_dir.join(&candidate)
    };
    let full_norm = normalize_lexical(&full);
    let base_norm = normalize_lexical(screenshots_dir);
    if !full_norm.starts_with(&base_norm) {
        return Err("Save path must be under the Screenshots directory".into());
    }

    Ok(screenshots_dir.join(safe_name))
}

/// Decode PNG base64 with a hard size cap (`MAX_IMAGE_BYTES`).
pub fn decode_png_bounded(png_base64: &str) -> Result<Vec<u8>, String> {
    // Base64 is ~4/3 of raw; reject absurd strings before decode.
    let max_b64 = MAX_IMAGE_BYTES
        .saturating_mul(4)
        .saturating_div(3)
        .saturating_add(64);
    if png_base64.len() > max_b64 {
        return Err(format!(
            "Image too large (base64 exceeds ~{} bytes)",
            MAX_IMAGE_BYTES
        ));
    }
    let bytes = STANDARD
        .decode(png_base64.trim())
        .map_err(|e| format!("Invalid base64 image: {e}"))?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "Image too large ({} bytes; max {})",
            bytes.len(),
            MAX_IMAGE_BYTES
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn sanitize_accepts_normal_screenshot_name() {
        let n = sanitize_screenshot_filename("Screenshot_2026-07-15_12-00-00.png").unwrap();
        assert_eq!(n, "Screenshot_2026-07-15_12-00-00.png");
    }

    #[test]
    fn sanitize_rejects_traversal_and_separators() {
        assert!(sanitize_screenshot_filename("../evil.png").is_err());
        assert!(sanitize_screenshot_filename("a/b.png").is_err());
        assert!(sanitize_screenshot_filename("a\\b.png").is_err());
        assert!(sanitize_screenshot_filename("/etc/passwd.png").is_err());
        assert!(sanitize_screenshot_filename("..").is_err());
        assert!(sanitize_screenshot_filename("").is_err());
        assert!(sanitize_screenshot_filename("noext").is_err());
        assert!(sanitize_screenshot_filename("bad name.png").is_err());
        assert!(sanitize_screenshot_filename("x\0.png").is_err());
    }

    #[test]
    fn resolve_none_uses_default_under_dir() {
        let base = PathBuf::from("/home/u/Pictures/Screenshots");
        let p = resolve_snip_save_path(&base, None, "Screenshot_1.png").unwrap();
        assert_eq!(p, base.join("Screenshot_1.png"));
    }

    #[test]
    fn resolve_basename_only() {
        let base = PathBuf::from("/home/u/Pictures/Screenshots");
        let p = resolve_snip_save_path(&base, Some("my_shot.png"), "Screenshot_1.png").unwrap();
        assert_eq!(p, base.join("my_shot.png"));
    }

    #[test]
    fn resolve_absolute_under_screenshots_ok() {
        let base = PathBuf::from("/home/u/Pictures/Screenshots");
        let p = resolve_snip_save_path(
            &base,
            Some("/home/u/Pictures/Screenshots/Screenshot_1.png"),
            "Screenshot_1.png",
        )
        .unwrap();
        assert_eq!(p, base.join("Screenshot_1.png"));
    }

    #[test]
    fn resolve_rejects_escape_paths() {
        let base = PathBuf::from("/home/u/Pictures/Screenshots");
        assert!(
            resolve_snip_save_path(&base, Some("/tmp/evil.png"), "Screenshot_1.png").is_err()
        );
        assert!(resolve_snip_save_path(
            &base,
            Some("/home/u/Pictures/Screenshots/../../.ssh/id_rsa.png"),
            "Screenshot_1.png",
        )
        .is_err());
        assert!(resolve_snip_save_path(
            &base,
            Some("../other.png"),
            "Screenshot_1.png",
        )
        .is_err());
    }

    #[test]
    fn is_under_dir_lexical() {
        let base = Path::new("/home/u/Pictures/Screenshots");
        assert!(is_under_dir(
            Path::new("/home/u/Pictures/Screenshots/a.png"),
            base
        ));
        assert!(!is_under_dir(Path::new("/tmp/a.png"), base));
        assert!(!is_under_dir(
            Path::new("/home/u/Pictures/Screenshots/../Secrets/x"),
            base
        ));
    }

    #[test]
    fn decode_png_bounded_rejects_oversize_string() {
        let huge = "A".repeat(MAX_IMAGE_BYTES.saturating_mul(2));
        assert!(decode_png_bounded(&huge).is_err());
    }

    #[test]
    fn decode_png_bounded_rejects_invalid_base64() {
        assert!(decode_png_bounded("!!!not-base64!!!").is_err());
    }

    #[test]
    fn decode_png_bounded_accepts_small_payload() {
        // "hi" as base64
        let bytes = decode_png_bounded("aGk=").unwrap();
        assert_eq!(bytes, b"hi");
    }
}
