use crate::clipboard::types::{ClipItem, ClipItemType};
use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

const MAX_HISTORY: usize = 100;
const MAX_TEXT_BYTES: usize = 1_048_576;
const MAX_IMAGE_BYTES: usize = 10_485_760;

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
        fs::create_dir_all(&data_dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&data_dir, fs::Permissions::from_mode(0o700))?;
        }

        let db_path = data_dir.join("history.db");
        let conn = Connection::open(db_path)?;
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

    pub fn insert_item(&self, item_type: ClipItemType, content: &str, preview: &str) -> Result<Option<ClipItem>, DbError> {
        let byte_len = content.len();
        let limit = match item_type {
            ClipItemType::Text => MAX_TEXT_BYTES,
            ClipItemType::Image => MAX_IMAGE_BYTES,
        };
        if byte_len > limit {
            return Ok(None);
        }

        if self.content_exists(content)? {
            return Ok(None);
        }

        let id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().timestamp_millis();
        self.conn.execute(
            "INSERT INTO items (id, item_type, content, preview, pinned, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            params![
                id,
                item_type.as_str(),
                content,
                preview,
                created_at
            ],
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

    fn content_exists(&self, content: &str) -> Result<bool, DbError> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM items WHERE content = ?1",
            params![content],
            |row| row.get(0),
        )?;
        Ok(count > 0)
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

    pub fn list_items(&self) -> Result<Vec<ClipItem>, DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, item_type, content, preview, pinned, created_at
             FROM items
             ORDER BY pinned DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let item_type: String = row.get(1)?;
            Ok(ClipItem {
                id: row.get(0)?,
                item_type: ClipItemType::from_str(&item_type),
                content: row.get(2)?,
                preview: row.get(3)?,
                pinned: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
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
        self.conn.execute("DELETE FROM items WHERE id = ?1", params![id])?;
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
}