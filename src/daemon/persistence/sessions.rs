use anyhow::Context;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::Database;

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedSessionData {
    #[serde(default)]
    vars: std::collections::HashMap<String, String>,
    #[serde(default)]
    profile: Option<String>,
}

impl Database {
    /// Upsert a session record.
    pub fn upsert_session(&self, session: &crate::SessionData) -> anyhow::Result<()> {
        let data_json = serde_json::to_string(&PersistedSessionData {
            vars: session.vars.clone(),
            profile: session.profile.clone(),
        })
        .unwrap_or_default();
        self.conn
            .execute(
                "INSERT INTO sessions (name, data_json, created_at, last_active)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(name) DO UPDATE SET
                     data_json = excluded.data_json,
                     last_active = excluded.last_active",
                params![
                    session.name,
                    data_json,
                    session.created_at as i64,
                    session.last_active as i64,
                ],
            )
            .context("failed to upsert session")?;
        Ok(())
    }

    /// Delete a session by name. Returns true if a row was removed.
    pub fn delete_session(&self, name: &str) -> anyhow::Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM sessions WHERE name = ?1", params![name])
            .context("failed to delete session")?;
        Ok(affected > 0)
    }

    /// Load all sessions from the database.
    pub fn load_sessions(&self) -> anyhow::Result<Vec<crate::SessionData>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, data_json, created_at, last_active FROM sessions ORDER BY name",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows
            .into_iter()
            .map(|(name, data_json, created_at, last_active)| {
                let parsed = parse_session_data(&data_json);
                crate::SessionData {
                    name,
                    vars: parsed.vars,
                    profile: parsed.profile,
                    created_at: created_at as u64,
                    last_active: last_active as u64,
                }
            })
            .collect())
    }
}

fn parse_session_data(data_json: &str) -> PersistedSessionData {
    if let Ok(parsed) = serde_json::from_str::<PersistedSessionData>(data_json) {
        return parsed;
    }
    let vars: std::collections::HashMap<String, String> =
        serde_json::from_str(data_json).unwrap_or_default();
    PersistedSessionData {
        vars,
        profile: None,
    }
}
