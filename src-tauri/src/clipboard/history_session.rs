//! Simulated clipboard history session for tests (no arboard / GTK / Tauri).

use crate::clipboard::monitor::preview_text;
use crate::clipboard::policy::{choose_capture, hash_text, CaptureKind};
use crate::clipboard::types::ClipItemType;
use crate::db::Database;
use std::path::PathBuf;

/// In-memory gate + temp DB — models monitor + clear/delete side effects.
pub struct HistorySession {
    pub db: Database,
    seen_text: String,
    seen_image: String,
    /// Content currently “on the system clipboard” for mark-live simulation.
    live_text: Option<String>,
    live_image_hash: Option<String>,
}

impl HistorySession {
    pub fn open_at(data_dir: PathBuf) -> Self {
        let db = Database::open_at(data_dir).expect("open_at");
        Self {
            db,
            seen_text: String::new(),
            seen_image: String::new(),
            live_text: None,
            live_image_hash: None,
        }
    }

    pub fn set_live_text(&mut self, text: impl Into<String>) {
        self.live_text = Some(text.into());
        self.live_image_hash = None;
    }

    pub fn set_live_image_hash(&mut self, hash: impl Into<String>) {
        self.live_image_hash = Some(hash.into());
    }

    /// Like monitor poll: only inserts when choose_capture says so.
    pub fn poll_ingest(&mut self) -> Option<ClipItemType> {
        let text = self.live_text.as_deref();
        let th = text.map(hash_text);
        let ih = self.live_image_hash.as_deref();

        let kind = choose_capture(
            text,
            th.as_deref(),
            ih,
            &self.seen_text,
            &self.seen_image,
        );

        match kind {
            CaptureKind::None => None,
            CaptureKind::Text => {
                let text = text.expect("text capture");
                let preview = preview_text(text);
                let item = self
                    .db
                    .insert_item(ClipItemType::Text, text, &preview)
                    .ok()
                    .flatten()?;
                self.seen_text = th.expect("text hash");
                if let Some(ih) = ih {
                    self.seen_image = ih.to_string();
                }
                let _ = item;
                Some(ClipItemType::Text)
            }
            CaptureKind::Image => {
                // Tests use a synthetic image content string keyed by hash.
                let hash = ih.expect("image hash").to_string();
                let content = format!("data:image/png;base64,{hash}");
                let item = self
                    .db
                    .insert_item(ClipItemType::Image, &content, &content)
                    .ok()
                    .flatten()?;
                self.seen_image = hash;
                if let Some(th) = th {
                    self.seen_text = th;
                }
                let _ = item;
                Some(ClipItemType::Image)
            }
        }
    }

    /// User copies text (updates live clipboard then poll).
    pub fn copy_text(&mut self, text: &str) -> bool {
        self.set_live_text(text);
        self.poll_ingest() == Some(ClipItemType::Text)
    }

    /// User copies an “image” identified only by a stable hash string.
    pub fn copy_image_hash(&mut self, hash: &str, incidental_text: Option<&str>) -> bool {
        self.live_image_hash = Some(hash.to_string());
        self.live_text = incidental_text.map(str::to_string);
        self.poll_ingest() == Some(ClipItemType::Image)
    }

    /// Clear unpinned + mark live clipboard as seen (no re-insert until change).
    pub fn clear_all_unpinned(&mut self) {
        self.db.clear_unpinned().expect("clear");
        self.mark_live_as_seen();
    }

    pub fn mark_live_as_seen(&mut self) {
        if let Some(ref t) = self.live_text {
            self.seen_text = hash_text(t);
        }
        if let Some(ref h) = self.live_image_hash {
            self.seen_image = h.clone();
        }
    }

    pub fn delete(&mut self, id: &str) {
        self.db.delete_item(id).expect("delete");
        self.mark_live_as_seen();
    }

    pub fn pin(&mut self, id: &str) {
        self.db.set_pinned(id, true).expect("pin");
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.db
            .list_summaries()
            .expect("list")
            .into_iter()
            .map(|s| s.id)
            .collect()
    }

    pub fn list_previews(&self) -> Vec<String> {
        self.db
            .list_summaries()
            .expect("list")
            .into_iter()
            .map(|s| s.preview)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::types::ClipItemType;
    use std::thread;
    use std::time::Duration;

    fn session() -> (tempfile::TempDir, HistorySession) {
        let dir = tempfile::tempdir().unwrap();
        let s = HistorySession::open_at(dir.path().to_path_buf());
        (dir, s)
    }

    #[test]
    fn copy_text_shows_in_list() {
        let (_d, mut s) = session();
        assert!(s.copy_text("hello world"));
        assert_eq!(s.list_previews(), vec!["hello world".to_string()]);
    }

    #[test]
    fn clear_all_stays_empty_for_same_live_clipboard() {
        let (_d, mut s) = session();
        assert!(s.copy_text("sticky"));
        assert_eq!(s.list_ids().len(), 1);

        s.clear_all_unpinned();
        assert!(s.list_ids().is_empty());

        // Simulated poll with same live content must not re-insert (no flash).
        assert!(s.poll_ingest().is_none());
        assert!(s.list_ids().is_empty());
    }

    #[test]
    fn copy_after_clear_all_inserts_new_content() {
        let (_d, mut s) = session();
        assert!(s.copy_text("before clear"));
        s.clear_all_unpinned();
        assert!(s.list_ids().is_empty());

        assert!(s.copy_text("after clear"));
        assert_eq!(s.list_previews(), vec!["after clear".to_string()]);
    }

    #[test]
    fn copy_same_text_after_clear_does_not_flash_but_db_allows_reinsert_if_gate_opens() {
        let (_d, mut s) = session();
        assert!(s.copy_text("same"));
        s.clear_all_unpinned();
        // Gate still holds "same" as seen → no flash
        assert!(!s.copy_text("same"));
        assert!(s.list_ids().is_empty());

        // If user copies something else then back, it appears
        assert!(s.copy_text("other"));
        assert!(s.copy_text("same"));
        let previews = s.list_previews();
        assert!(previews.contains(&"same".to_string()));
        assert_eq!(previews[0], "same");
    }

    #[test]
    fn pin_survives_clear_and_new_copy_works() {
        let (_d, mut s) = session();
        assert!(s.copy_text("pinned-note"));
        let pin_id = s.list_ids()[0].clone();
        s.pin(&pin_id);
        assert!(s.copy_text("temp"));
        s.clear_all_unpinned();

        let list = s.db.list_summaries().unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].pinned);
        assert_eq!(list[0].preview, "pinned-note");

        assert!(s.copy_text("fresh"));
        let previews = s.list_previews();
        assert!(previews.iter().any(|p| p == "fresh"));
        assert!(previews.iter().any(|p| p == "pinned-note"));
    }

    #[test]
    fn edit_text_updates_content_for_paste() {
        let (_d, mut s) = session();
        assert!(s.copy_text("old body"));
        let id = s.list_ids()[0].clone();
        thread::sleep(Duration::from_millis(5));
        assert!(s.copy_text("other"));
        thread::sleep(Duration::from_millis(5));

        let updated = s
            .db
            .update_text_item(&id, "edited body", "edited body")
            .unwrap()
            .unwrap();
        assert_eq!(updated.content, "edited body");
        let loaded = s.db.get_item(&id).unwrap().unwrap();
        assert_eq!(loaded.content, "edited body");
        // Paste-from-list would use full content
        assert_eq!(loaded.content, "edited body");
    }

    #[test]
    fn promote_on_paste_from_list() {
        let (_d, mut s) = session();
        assert!(s.copy_text("A"));
        let a_id = s.list_ids()[0].clone();
        thread::sleep(Duration::from_millis(5));
        assert!(s.copy_text("B"));
        assert_eq!(s.list_previews()[0], "B");

        thread::sleep(Duration::from_millis(5));
        s.db.promote_item(&a_id).unwrap().unwrap();
        assert_eq!(s.list_previews()[0], "A");
        // Content still available for paste
        let item = s.db.get_item(&a_id).unwrap().unwrap();
        assert_eq!(item.content, "A");
    }

    #[test]
    fn delete_then_copy_different_then_same() {
        let (_d, mut s) = session();
        assert!(s.copy_text("x"));
        let id = s.list_ids()[0].clone();
        s.delete(&id);
        assert!(s.list_ids().is_empty());
        // Same live clipboard after delete is marked seen → no flash
        assert!(s.poll_ingest().is_none());
        assert!(s.copy_text("y"));
        assert!(s.copy_text("x"));
        assert!(s.list_previews().contains(&"x".to_string()));
    }

    #[test]
    fn emoji_copy_and_paste_content() {
        let (_d, mut s) = session();
        let emoji = "😀🎉";
        assert!(s.copy_text(emoji));
        let id = s.list_ids()[0].clone();
        let item = s.db.get_item(&id).unwrap().unwrap();
        assert_eq!(item.content, emoji);
        assert_eq!(item.item_type, ClipItemType::Text);
    }

    #[test]
    fn image_copy_with_incidental_url() {
        let (_d, mut s) = session();
        assert!(s.copy_image_hash("imghash1", Some("https://cdn.example/a.png")));
        let list = s.db.list_summaries().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].item_type, ClipItemType::Image);
    }

    #[test]
    fn real_text_wins_over_stale_and_new_image() {
        let (_d, mut s) = session();
        // Seed seen image
        assert!(s.copy_image_hash("oldimg", Some("https://x/y.png")));
        // User copies real multi-line text while image target still "present"
        s.live_image_hash = Some("oldimg".into());
        assert!(s.copy_text("fn main() {\n  println!(\"hi\");\n}"));
        let list = s.db.list_summaries().unwrap();
        assert!(list.iter().any(|i| i.item_type == ClipItemType::Text));
        assert_eq!(list[0].item_type, ClipItemType::Text);
    }

    #[test]
    fn successive_copies_order_newest_first() {
        let (_d, mut s) = session();
        assert!(s.copy_text("first"));
        thread::sleep(Duration::from_millis(5));
        assert!(s.copy_text("second"));
        thread::sleep(Duration::from_millis(5));
        assert!(s.copy_text("third"));
        assert_eq!(
            s.list_previews(),
            vec![
                "third".to_string(),
                "second".to_string(),
                "first".to_string()
            ]
        );
    }
}
