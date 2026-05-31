use anyhow::Context;
use rusqlite::params;

use super::{unix_now, Database};

impl Database {
    /// Insert or update a blackboard key-value entry.
    pub fn upsert_blackboard(
        &self,
        key: &str,
        namespace: &str,
        value_json: &str,
        ttl: Option<u64>,
    ) -> anyhow::Result<()> {
        let now = unix_now();
        let ttl_val: Option<i64> = ttl.map(|v| v as i64);
        self.conn
            .execute(
                "INSERT INTO blackboard (key, namespace, value_json, ttl, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(key, namespace) DO UPDATE SET
                     value_json = excluded.value_json,
                     ttl = excluded.ttl,
                     updated_at = excluded.updated_at",
                rusqlite::params![key, namespace, value_json, ttl_val, now, now],
            )
            .context("failed to upsert blackboard entry")?;
        Ok(())
    }

    /// Retrieve a blackboard value by key and namespace.
    pub fn get_blackboard(&self, key: &str, namespace: &str) -> anyhow::Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value_json, ttl, updated_at FROM blackboard WHERE key = ?1 AND namespace = ?2",
        )?;
        let mut rows = stmt.query_map(params![key, namespace], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;

        let Some(result) = rows.next() else {
            return Ok(None);
        };
        let (value, ttl, updated_at) = result?;

        // Check TTL expiry
        if let Some(ttl_secs) = ttl {
            let now = unix_now();
            let expiry = (updated_at as u64).saturating_add(ttl_secs as u64);
            if (now as u64) > expiry {
                let _ = self.delete_blackboard(key, namespace);
                return Ok(None);
            }
        }

        Ok(Some(value))
    }

    /// Delete a blackboard entry. Returns true if a row was actually removed.
    pub fn delete_blackboard(&self, key: &str, namespace: &str) -> anyhow::Result<bool> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM blackboard WHERE key = ?1 AND namespace = ?2",
                params![key, namespace],
            )
            .context("failed to delete blackboard entry")?;
        Ok(affected > 0)
    }

    /// List all keys in a given namespace.
    pub fn blackboard_keys(&self, namespace: &str) -> anyhow::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT key FROM blackboard WHERE namespace = ?1 ORDER BY key")?;
        let keys = stmt
            .query_map(params![namespace], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(keys)
    }
}
