use crate::clipboard::types::{ClipItem, ClipItemSummary, ClipItemType};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

pub const MAX_HISTORY: usize = 100;
pub const MAX_TEXT_BYTES: usize = 1_048_576;
pub const MAX_IMAGE_BYTES: usize = 10_485_760;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Database {
    conn: Connection,
    data_dir: PathBuf,
}

impl Database {
    pub fn open() -> Result<Self, DbError> {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("clipnpaste");
        Self::open_at(data_dir)
    }

    /// Open (or create) the history database under an arbitrary data directory.
    pub fn open_at(data_dir: PathBuf) -> Result<Self, DbError> {
        fs::create_dir_all(&data_dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&data_dir, fs::Permissions::from_mode(0o700))?;
        }

        let db_path = data_dir.join("history.db");
        let conn = Connection::open(&db_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&db_path, fs::Permissions::from_mode(0o600));
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS items (
                id TEXT PRIMARY KEY,
                item_type TEXT NOT NULL,
                content TEXT NOT NULL,
                preview TEXT NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_items_pinned_created
                ON items (pinned DESC, created_at DESC);",
        )?;

        Ok(Self { conn, data_dir })
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Insert a new history item, or bump an existing matching content entry to the top.
    pub fn insert_item(
        &self,
        item_type: ClipItemType,
        content: &str,
        preview: &str,
    ) -> Result<Option<ClipItem>, DbError> {
        let byte_len = content.len();
        let limit = match item_type {
            ClipItemType::Text => MAX_TEXT_BYTES,
            ClipItemType::Image => MAX_IMAGE_BYTES,
        };
        if byte_len > limit {
            return Ok(None);
        }

        if let Some(existing) = self.find_by_content(content)? {
            let created_at = chrono::Utc::now().timestamp_millis();
            self.conn.execute(
                "UPDATE items SET created_at = ?1, preview = ?2, item_type = ?3 WHERE id = ?4",
                params![created_at, preview, item_type.as_str(), existing.id],
            )?;
            return Ok(Some(ClipItem {
                id: existing.id,
                item_type,
                preview: preview.to_string(),
                content: content.to_string(),
                pinned: existing.pinned,
                created_at,
            }));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().timestamp_millis();
        self.conn.execute(
            "INSERT INTO items (id, item_type, content, preview, pinned, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            params![id, item_type.as_str(), content, preview, created_at],
        )?;
        self.trim_history()?;

        Ok(Some(ClipItem {
            id,
            item_type,
            preview: preview.to_string(),
            content: content.to_string(),
            pinned: false,
            created_at,
        }))
    }

    /// Bump an existing item to the top of history by id (Windows-style promote-on-select).
    pub fn promote_item(&self, id: &str) -> Result<Option<ClipItem>, DbError> {
        let Some(item) = self.get_item(id)? else {
            return Ok(None);
        };
        let created_at = chrono::Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE items SET created_at = ?1 WHERE id = ?2",
            params![created_at, id],
        )?;
        Ok(Some(ClipItem {
            created_at,
            ..item
        }))
    }

    fn find_by_content(&self, content: &str) -> Result<Option<ClipItemSummary>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item_type, preview, pinned, created_at
             FROM items WHERE content = ?1 LIMIT 1",
        )?;
        let item = stmt
            .query_row(params![content], |row| {
                let item_type: String = row.get(1)?;
                Ok(ClipItemSummary {
                    id: row.get(0)?,
                    item_type: ClipItemType::from_str(&item_type),
                    preview: row.get(2)?,
                    pinned: row.get::<_, i64>(3)? != 0,
                    created_at: row.get(4)?,
                })
            })
            .optional()?;
        Ok(item)
    }

    fn trim_history(&self) -> Result<(), DbError> {
        self.conn.execute(
            "DELETE FROM items WHERE id IN (
                SELECT id FROM items WHERE pinned = 0
                ORDER BY created_at DESC
                LIMIT -1 OFFSET ?1
            )",
            params![MAX_HISTORY as i64],
        )?;
        Ok(())
    }

    pub fn list_summaries(&self) -> Result<Vec<ClipItemSummary>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item_type, preview, pinned, created_at
             FROM items
             ORDER BY pinned DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let item_type: String = row.get(1)?;
            Ok(ClipItemSummary {
                id: row.get(0)?,
                item_type: ClipItemType::from_str(&item_type),
                preview: row.get(2)?,
                pinned: row.get::<_, i64>(3)? != 0,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), DbError> {
        self.conn.execute(
            "UPDATE items SET pinned = ?1 WHERE id = ?2",
            params![pinned as i64, id],
        )?;
        Ok(())
    }

    pub fn delete_item(&self, id: &str) -> Result<(), DbError> {
        self.conn
            .execute("DELETE FROM items WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_unpinned(&self) -> Result<(), DbError> {
        self.conn.execute("DELETE FROM items WHERE pinned = 0", [])?;
        Ok(())
    }

    pub fn get_item(&self, id: &str) -> Result<Option<ClipItem>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item_type, content, preview, pinned, created_at
             FROM items WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let item_type: String = row.get(1)?;
            return Ok(Some(ClipItem {
                id: row.get(0)?,
                item_type: ClipItemType::from_str(&item_type),
                content: row.get(2)?,
                preview: row.get(3)?,
                pinned: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
            }));
        }
        Ok(None)
    }

    /// Replace text content/preview for a text history item and bump it to the top.
    /// Returns `None` if the id is missing, not text, or content exceeds the size limit.
    pub fn update_text_item(
        &self,
        id: &str,
        content: &str,
        preview: &str,
    ) -> Result<Option<ClipItem>, DbError> {
        if content.len() > MAX_TEXT_BYTES {
            return Ok(None);
        }
        let Some(existing) = self.get_item(id)? else {
            return Ok(None);
        };
        if existing.item_type != ClipItemType::Text {
            return Ok(None);
        }
        let created_at = chrono::Utc::now().timestamp_millis();
        self.conn.execute(
            "UPDATE items SET content = ?1, preview = ?2, created_at = ?3 WHERE id = ?4",
            params![content, preview, created_at, id],
        )?;
        Ok(Some(ClipItem {
            id: existing.id,
            item_type: ClipItemType::Text,
            preview: preview.to_string(),
            content: content.to_string(),
            pinned: existing.pinned,
            created_at,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::types::ClipItemType;
    use std::thread;
    use std::time::Duration;

    fn temp_db() -> (tempfile::TempDir, Database) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = Database::open_at(dir.path().to_path_buf()).expect("open_at");
        (dir, db)
    }

    #[test]
    fn insert_text_appears_in_summaries() {
        let (_dir, db) = temp_db();
        let item = db
            .insert_item(ClipItemType::Text, "hello", "hello")
            .unwrap()
            .expect("inserted");
        let list = db.list_summaries().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, item.id);
        assert_eq!(list[0].preview, "hello");
    }

    #[test]
    fn insert_touch_bumps_to_top() {
        let (_dir, db) = temp_db();
        let a = db
            .insert_item(ClipItemType::Text, "A", "A")
            .unwrap()
            .unwrap();
        thread::sleep(Duration::from_millis(5));
        let _b = db
            .insert_item(ClipItemType::Text, "B", "B")
            .unwrap()
            .unwrap();
        thread::sleep(Duration::from_millis(5));
        let a2 = db
            .insert_item(ClipItemType::Text, "A", "A")
            .unwrap()
            .unwrap();
        assert_eq!(a.id, a2.id);
        let list = db.list_summaries().unwrap();
        assert_eq!(list[0].id, a.id);
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn promote_item_by_id() {
        let (_dir, db) = temp_db();
        let a = db
            .insert_item(ClipItemType::Text, "A", "A")
            .unwrap()
            .unwrap();
        thread::sleep(Duration::from_millis(5));
        let b = db
            .insert_item(ClipItemType::Text, "B", "B")
            .unwrap()
            .unwrap();
        assert_eq!(db.list_summaries().unwrap()[0].id, b.id);
        thread::sleep(Duration::from_millis(5));
        let promoted = db.promote_item(&a.id).unwrap().unwrap();
        assert_eq!(promoted.id, a.id);
        assert!(promoted.created_at >= b.created_at);
        assert_eq!(db.list_summaries().unwrap()[0].id, a.id);
    }

    #[test]
    fn promote_missing_returns_none() {
        let (_dir, db) = temp_db();
        assert!(db.promote_item("nope").unwrap().is_none());
    }

    #[test]
    fn pin_survives_clear_unpinned() {
        let (_dir, db) = temp_db();
        let pinned = db
            .insert_item(ClipItemType::Text, "keep", "keep")
            .unwrap()
            .unwrap();
        let _gone = db
            .insert_item(ClipItemType::Text, "drop", "drop")
            .unwrap()
            .unwrap();
        db.set_pinned(&pinned.id, true).unwrap();
        db.clear_unpinned().unwrap();
        let list = db.list_summaries().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, pinned.id);
        assert!(list[0].pinned);
    }

    #[test]
    fn delete_item_removes() {
        let (_dir, db) = temp_db();
        let item = db
            .insert_item(ClipItemType::Text, "x", "x")
            .unwrap()
            .unwrap();
        db.delete_item(&item.id).unwrap();
        assert!(db.list_summaries().unwrap().is_empty());
        assert!(db.get_item(&item.id).unwrap().is_none());
    }

    #[test]
    fn oversized_text_rejected() {
        let (_dir, db) = temp_db();
        let huge = "a".repeat(MAX_TEXT_BYTES + 1);
        assert!(db
            .insert_item(ClipItemType::Text, &huge, "preview")
            .unwrap()
            .is_none());
    }

    #[test]
    fn oversized_image_rejected() {
        let (_dir, db) = temp_db();
        let huge = "x".repeat(MAX_IMAGE_BYTES + 1);
        assert!(db
            .insert_item(ClipItemType::Image, &huge, "preview")
            .unwrap()
            .is_none());
    }

    #[test]
    fn empty_text_insert_ok() {
        let (_dir, db) = temp_db();
        let item = db
            .insert_item(ClipItemType::Text, "", "")
            .unwrap()
            .expect("empty text allowed");
        assert_eq!(item.content, "");
    }

    #[test]
    fn delete_missing_is_ok() {
        let (_dir, db) = temp_db();
        db.delete_item("missing-id").unwrap();
    }

    #[test]
    fn get_item_returns_content() {
        let (_dir, db) = temp_db();
        let item = db
            .insert_item(ClipItemType::Text, "full body", "preview")
            .unwrap()
            .unwrap();
        let loaded = db.get_item(&item.id).unwrap().unwrap();
        assert_eq!(loaded.content, "full body");
        assert_eq!(loaded.preview, "preview");
    }

    #[test]
    fn trim_history_keeps_pinned() {
        let (_dir, db) = temp_db();
        let pinned = db
            .insert_item(ClipItemType::Text, "pinned-item", "pinned")
            .unwrap()
            .unwrap();
        db.set_pinned(&pinned.id, true).unwrap();
        for i in 0..(MAX_HISTORY + 20) {
            let content = format!("item-{i}");
            db.insert_item(ClipItemType::Text, &content, &content)
                .unwrap()
                .unwrap();
        }
        let list = db.list_summaries().unwrap();
        assert!(list.iter().any(|s| s.id == pinned.id && s.pinned));
        // Pinned + at most MAX_HISTORY unpinned
        let unpinned = list.iter().filter(|s| !s.pinned).count();
        assert!(unpinned <= MAX_HISTORY);
        assert!(list.len() <= MAX_HISTORY + 1);
    }

    #[test]
    fn update_text_item_changes_content_and_preview() {
        let (_dir, db) = temp_db();
        let item = db
            .insert_item(ClipItemType::Text, "old body", "old body")
            .unwrap()
            .unwrap();
        thread::sleep(Duration::from_millis(5));
        let _other = db
            .insert_item(ClipItemType::Text, "other", "other")
            .unwrap()
            .unwrap();
        thread::sleep(Duration::from_millis(5));
        let updated = db
            .update_text_item(&item.id, "new body", "new body")
            .unwrap()
            .unwrap();
        assert_eq!(updated.content, "new body");
        assert_eq!(updated.preview, "new body");
        assert_eq!(updated.id, item.id);
        let loaded = db.get_item(&item.id).unwrap().unwrap();
        assert_eq!(loaded.content, "new body");
        // Edited item is promoted to top among unpinned.
        assert_eq!(db.list_summaries().unwrap()[0].id, item.id);
    }

    #[test]
    fn update_text_item_rejects_image() {
        let (_dir, db) = temp_db();
        let item = db
            .insert_item(
                ClipItemType::Image,
                "data:image/png;base64,xx",
                "data:image/png;base64,xx",
            )
            .unwrap()
            .unwrap();
        assert!(db
            .update_text_item(&item.id, "nope", "nope")
            .unwrap()
            .is_none());
    }

    #[test]
    fn update_text_item_rejects_missing_and_oversized() {
        let (_dir, db) = temp_db();
        assert!(db
            .update_text_item("missing", "x", "x")
            .unwrap()
            .is_none());
        let item = db
            .insert_item(ClipItemType::Text, "ok", "ok")
            .unwrap()
            .unwrap();
        let huge = "a".repeat(MAX_TEXT_BYTES + 1);
        assert!(db
            .update_text_item(&item.id, &huge, "p")
            .unwrap()
            .is_none());
    }
}
