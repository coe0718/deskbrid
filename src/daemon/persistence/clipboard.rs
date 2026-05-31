use crate::protocol::ClipboardHistoryEntry;
use anyhow::Context;
use rusqlite::params;

use super::{Database, unix_now};

impl Database {
    /// Insert a clipboard entry and return its row id.
    pub fn insert_clipboard(&self, text: &str, source: Option<&str>) -> anyhow::Result<i64> {
        let now = unix_now();
        self.conn
            .execute(
                "INSERT INTO clipboard_history (text, source, copied_at) VALUES (?1, ?2, ?3)",
                params![text, source, now],
            )
            .context("failed to insert clipboard entry")?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Retrieve the most recent clipboard entries, optionally filtered by a text query.
    pub fn get_clipboard_history(
        &self,
        limit: usize,
        query: Option<&str>,
    ) -> anyhow::Result<Vec<ClipboardHistoryEntry>> {
        let rows = if let Some(q) = query {
            let like = format!("%{}%", q);
            let mut stmt = self.conn.prepare(
                "SELECT id, text, source, copied_at FROM clipboard_history
                 WHERE text LIKE ?1
                 ORDER BY id DESC LIMIT ?2",
            )?;
            stmt.query_map(params![like, limit as i64], |row| {
                Ok(CbRow {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    source: row.get(2)?,
                    copied_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT id, text, source, copied_at FROM clipboard_history
                 ORDER BY id DESC LIMIT ?1",
            )?;
            stmt.query_map(params![limit as i64], |row| {
                Ok(CbRow {
                    id: row.get(0)?,
                    text: row.get(1)?,
                    source: row.get(2)?,
                    copied_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        Ok(rows
            .into_iter()
            .map(|r| {
                let len = r.text.len();
                ClipboardHistoryEntry {
                    id: r.id as u64,
                    timestamp: r.copied_at as u64,
                    text: r.text,
                    size: len,
                    source: r.source.unwrap_or_default(),
                }
            })
            .collect())
    }

    /// Delete all clipboard history rows.
    pub fn clear_clipboard(&self) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM clipboard_history", [])
            .context("failed to clear clipboard history")?;
        Ok(())
    }
}

struct CbRow {
    id: i64,
    text: String,
    source: Option<String>,
    copied_at: i64,
}
