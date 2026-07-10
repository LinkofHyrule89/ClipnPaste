//! Pure clipboard capture policy (no arboard / GTK).
//! Used by the live monitor and unit tests.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    Text,
    Image,
    None,
}

/// Decide what a monitor poll should store given optional live formats and seen hashes.
pub fn choose_capture(
    text: Option<&str>,
    text_hash: Option<&str>,
    image_hash: Option<&str>,
    seen_text: &str,
    seen_image: &str,
) -> CaptureKind {
    let text_new = text_hash.map(|h| h != seen_text).unwrap_or(false);
    let image_new = image_hash.map(|h| h != seen_image).unwrap_or(false);

    if !text_new && !image_new {
        return CaptureKind::None;
    }

    // New text (Ctrl+C/X) wins unless text is incidental and image is also new
    // (browser/file-manager image copy with URL/path text).
    let take_image = image_new
        && image_hash.is_some()
        && (!text_new || text.map(is_incidental_image_text).unwrap_or(true));

    if take_image {
        CaptureKind::Image
    } else if text_new {
        CaptureKind::Text
    } else {
        CaptureKind::None
    }
}

/// Text that often accompanies an image copy (URL, path) rather than a real selection.
pub fn is_incidental_image_text(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return true;
    }
    if t.starts_with("http://") || t.starts_with("https://") || t.starts_with("file://") {
        return true;
    }
    if !t.contains('\n') && t.len() < 260 && (t.contains('/') || t.contains('\\')) {
        return true;
    }
    false
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

pub fn hash_text(text: &str) -> String {
    hash_bytes(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_when_both_already_seen() {
        let th = hash_text("hello");
        let ih = "imagehash";
        assert_eq!(
            choose_capture(Some("hello"), Some(&th), Some(ih), &th, ih),
            CaptureKind::None
        );
    }

    #[test]
    fn new_text_chosen_when_stale_image_seen() {
        let th = hash_text("new paste");
        let ih = "oldimage";
        assert_eq!(
            choose_capture(Some("new paste"), Some(&th), Some(ih), "oldtext", ih),
            CaptureKind::Text
        );
    }

    #[test]
    fn new_image_with_url_text_chosen() {
        let th = hash_text("https://example.com/a.png");
        let ih = "newimage";
        assert_eq!(
            choose_capture(
                Some("https://example.com/a.png"),
                Some(&th),
                Some(ih),
                "",
                ""
            ),
            CaptureKind::Image
        );
    }

    #[test]
    fn new_real_text_wins_over_new_image() {
        let text = "Hello from clipboard\nsecond line";
        let th = hash_text(text);
        let ih = "newimage";
        assert_eq!(
            choose_capture(Some(text), Some(&th), Some(ih), "", ""),
            CaptureKind::Text
        );
    }

    #[test]
    fn only_new_image() {
        assert_eq!(
            choose_capture(None, None, Some("img1"), "", ""),
            CaptureKind::Image
        );
    }

    #[test]
    fn only_new_text() {
        let th = hash_text("only text");
        assert_eq!(
            choose_capture(Some("only text"), Some(&th), None, "", ""),
            CaptureKind::Text
        );
    }

    #[test]
    fn after_mark_seen_same_text_not_new() {
        let th = hash_text("A");
        assert_eq!(
            choose_capture(Some("A"), Some(&th), None, &th, ""),
            CaptureKind::None
        );
        let th_b = hash_text("B");
        assert_eq!(
            choose_capture(Some("B"), Some(&th_b), None, &th, ""),
            CaptureKind::Text
        );
    }

    #[test]
    fn incidental_image_text_detection() {
        assert!(is_incidental_image_text("https://example.com/a.png"));
        assert!(is_incidental_image_text("file:///tmp/shot.png"));
        assert!(is_incidental_image_text("/home/user/Pictures/x.png"));
        assert!(!is_incidental_image_text("Hello from clipboard"));
        assert!(!is_incidental_image_text("line one\nline two"));
    }
}
