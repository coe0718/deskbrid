use anyhow::Context;
use rusqlite::params;

use super::{unix_now, Database};

impl Database {
    /// Save (insert or replace) a macro.
    pub fn save_macro(&self, name: &str, actions_json: &str) -> anyhow::Result<()> {
        let now = unix_now();
        self.conn
            .execute(
                "INSERT INTO macros (name, actions_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(name) DO UPDATE SET
                     actions_json = excluded.actions_json,
                     updated_at = excluded.updated_at",
                params![name, actions_json, now, now],
            )
            .context("failed to save macro")?;
        Ok(())
    }

    /// Load a macro's actions JSON by name.
    pub fn load_macro(&self, name: &str) -> anyhow::Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT actions_json FROM macros WHERE name = ?1")?;
        let mut rows = stmt.query_map(params![name], |row| row.get(0))?;
        Ok(rows.next().transpose()?)
    }

    /// List all saved macro names.
    pub fn list_macros(&self) -> anyhow::Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM macros ORDER BY name")?;
        let names = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(names)
    }

    /// Delete a macro by name. Returns true if a row was removed.
    pub fn delete_macro(&self, name: &str) -> anyhow::Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM macros WHERE name = ?1", params![name])
            .context("failed to delete macro")?;
        Ok(affected > 0)
    }
}
