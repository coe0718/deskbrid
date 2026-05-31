use crate::protocol::{AuditEntry, ClipboardHistoryEntry, EventTrigger};
use anyhow::Context;
use rusqlite::{Connection, params};

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) the SQLite database at ~/.local/share/deskbrid/deskbrid.db.
    /// Enables WAL mode and runs schema initialization.
    pub fn open() -> anyhow::Result<Self> {
        let data_dir = dirs::data_dir()
            .context("could not determine XDG data directory")?
            .join("deskbrid");
        std::fs::create_dir_all(&data_dir).context("failed to create deskbrid data directory")?;
        let db_path = data_dir.join("deskbrid.db");

        let conn = Connection::open(&db_path).context("failed to open SQLite database")?;

        // Enable WAL mode for better concurrent read/write performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .context("failed to set WAL journal mode")?;

        let db = Self { conn };
        db.init_db()?;

        Ok(db)
    }

    /// Open an in-memory database (fallback when the on-disk DB is unavailable).
    pub fn memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory().context("failed to open in-memory database")?;
        let db = Self { conn };
        db.init_db()?;
        Ok(db)
    }

    fn init_db(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                source TEXT,
                copied_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY,
                seq INTEGER NOT NULL,
                uid INTEGER NOT NULL,
                action TEXT NOT NULL,
                params TEXT,
                status TEXT NOT NULL,
                duration_ms INTEGER,
                timestamp INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS notifications (
                id INTEGER PRIMARY KEY,
                app_name TEXT NOT NULL,
                title TEXT NOT NULL,
                body TEXT,
                urgency TEXT DEFAULT 'normal',
                actions TEXT,
                timestamp INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS macros (
                name TEXT PRIMARY KEY,
                actions_json TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cron_jobs (
                name TEXT PRIMARY KEY,
                interval_secs INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                action_params TEXT,
                last_run INTEGER,
                enabled INTEGER NOT NULL DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS blackboard (
                key TEXT NOT NULL,
                namespace TEXT NOT NULL DEFAULT 'default',
                value_json TEXT NOT NULL,
                ttl INTEGER,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (key, namespace)
            );
            CREATE TABLE IF NOT EXISTS sessions (
                name TEXT PRIMARY KEY,
                data_json TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                last_active INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                trigger_json TEXT NOT NULL,
                condition_json TEXT,
                action_type TEXT NOT NULL,
                action_params TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                max_fires INTEGER,
                cooldown_ms INTEGER
            );",
        )?;
        Ok(())
    }

    // ── Clipboard ────────────────────────────────────────

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

    // ── Audit ────────────────────────────────────────────

    /// Persist an audit entry.
    pub fn insert_audit(&self, entry: &AuditEntry) -> anyhow::Result<()> {
        let params_json = audit_params_json(entry);
        self.conn
            .execute(
                "INSERT OR REPLACE INTO audit_log (id, seq, uid, action, params, status, duration_ms, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    entry.id as i64,
                    entry.seq as i64,
                    entry.peer_uid,
                    entry.action_type,
                    params_json,
                    entry.status,
                    entry.duration_ms as i64,
                    entry.timestamp as i64,
                ],
            )
            .context("failed to insert audit entry")?;
        Ok(())
    }

    /// Retrieve audit log entries, optionally filtered by action type and/or status.
    pub fn get_audit_log(
        &self,
        limit: usize,
        action_type: Option<&str>,
        status: Option<&str>,
    ) -> anyhow::Result<Vec<AuditEntry>> {
        let mut sql = String::from(
            "SELECT id, seq, uid, action, params, status, duration_ms, timestamp FROM audit_log WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(at) = action_type {
            sql.push_str(" AND action = ?");
            param_values.push(Box::new(at.to_string()));
        }
        if let Some(st) = status {
            sql.push_str(" AND status = ?");
            param_values.push(Box::new(st.to_string()));
        }
        sql.push_str(" ORDER BY id DESC LIMIT ?");
        param_values.push(Box::new(limit as i64));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params_ref.as_slice(), |row| {
                Ok(AuditRow {
                    id: row.get(0)?,
                    seq: row.get(1)?,
                    uid: row.get(2)?,
                    action: row.get(3)?,
                    params: row.get(4)?,
                    status: row.get(5)?,
                    duration_ms: row.get(6)?,
                    timestamp: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let (error, dry_run) = parse_audit_params(&r.params);
                AuditEntry {
                    id: r.id as u64,
                    timestamp: r.timestamp as u64,
                    seq: r.seq as u64,
                    peer_uid: r.uid as u32,
                    action_type: r.action,
                    status: r.status,
                    duration_ms: r.duration_ms.unwrap_or(0) as u64,
                    error,
                    dry_run,
                }
            })
            .collect())
    }

    /// Delete all audit log rows.
    pub fn clear_audit(&self) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM audit_log", [])
            .context("failed to clear audit log")?;
        Ok(())
    }

    // ── Notifications ────────────────────────────────────

    /// Insert a notification and return its row id.
    pub fn insert_notification(
        &self,
        app_name: &str,
        title: &str,
        body: Option<&str>,
        urgency: Option<&str>,
        actions: Option<&[String]>,
        timestamp: u64,
    ) -> anyhow::Result<i64> {
        let actions_json = actions.map(serde_json::to_string).transpose()?;
        self.conn
            .execute(
                "INSERT INTO notifications (app_name, title, body, urgency, actions, timestamp)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    app_name,
                    title,
                    body,
                    urgency.unwrap_or("normal"),
                    actions_json,
                    timestamp as i64,
                ],
            )
            .context("failed to insert notification")?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Retrieve recent notifications, optionally filtered by app name and a since-timestamp.
    pub fn get_notifications(
        &self,
        limit: usize,
        app_name: Option<&str>,
        since: Option<u64>,
    ) -> anyhow::Result<Vec<serde_json::Value>> {
        let mut sql = String::from(
            "SELECT id, app_name, title, body, urgency, actions, timestamp FROM notifications WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(an) = app_name {
            sql.push_str(" AND app_name = ?");
            param_values.push(Box::new(an.to_string()));
        }
        if let Some(ts) = since {
            sql.push_str(" AND timestamp >= ?");
            param_values.push(Box::new(ts as i64));
        }
        sql.push_str(" ORDER BY id DESC LIMIT ?");
        param_values.push(Box::new(limit as i64));

        let params_ref: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params_ref.as_slice(), |row| {
                let actions_str: Option<String> = row.get(5)?;
                let actions: serde_json::Value = if let Some(ref s) = actions_str {
                    serde_json::from_str(s).unwrap_or(serde_json::Value::Null)
                } else {
                    serde_json::Value::Null
                };
                Ok(serde_json::json!({
                    "id": row.get::<_, i64>(0)?,
                    "app_name": row.get::<_, String>(1)?,
                    "title": row.get::<_, String>(2)?,
                    "body": row.get::<_, Option<String>>(3)?,
                    "urgency": row.get::<_, String>(4)?,
                    "actions": actions,
                    "timestamp": row.get::<_, i64>(6)?,
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    /// Delete all notification rows.
    pub fn clear_notifications(&self) -> anyhow::Result<()> {
        self.conn
            .execute("DELETE FROM notifications", [])
            .context("failed to clear notifications")?;
        Ok(())
    }

    // ── Blackboard ───────────────────────────────────────

    /// Insert or update a blackboard key-value entry.
    pub fn upsert_blackboard(
        &self,
        key: &str,
        namespace: &str,
        value_json: &str,
        ttl: Option<u64>,
    ) -> anyhow::Result<()> {
        let now = unix_now();
        self.conn
            .execute(
                "INSERT INTO blackboard (key, namespace, value_json, ttl, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(key, namespace) DO UPDATE SET
                     value_json = excluded.value_json,
                     ttl = excluded.ttl,
                     updated_at = excluded.updated_at",
                params![key, namespace, value_json, ttl.map(|v| v as i64), now, now],
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
                // Expired — clean up and return None
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

    // ── Macros ───────────────────────────────────────────

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

    // ── Sessions (#31) ────────────────────────────────────

    /// Upsert a session record.
    pub fn upsert_session(&self, session: &crate::SessionData) -> anyhow::Result<()> {
        let vars_json = serde_json::to_string(&session.vars).unwrap_or_default();
        self.conn
            .execute(
                "INSERT INTO sessions (name, data_json, created_at, last_active)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(name) DO UPDATE SET
                     data_json = excluded.data_json,
                     last_active = excluded.last_active",
                params![
                    session.name,
                    vars_json,
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
                let vars: std::collections::HashMap<String, String> =
                    serde_json::from_str(&data_json).unwrap_or_default();
                crate::SessionData {
                    name,
                    vars,
                    created_at: created_at as u64,
                    last_active: last_active as u64,
                }
            })
            .collect())
    }

    // ── Rules (#83) ──────────────────────────────────────

    /// Upsert a rule into the database.
    pub fn upsert_rule(&self, rule: &crate::protocol::Rule) -> anyhow::Result<()> {
        let trigger_json = serde_json::to_string(&rule.trigger)?;
        let condition_json = rule
            .condition
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let action_params = serde_json::to_string(&rule.action_params).unwrap_or_default();
        self.conn.execute(
            "INSERT INTO rules (id, name, trigger_json, condition_json, action_type, action_params, enabled, max_fires, cooldown_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                trigger_json = excluded.trigger_json,
                condition_json = excluded.condition_json,
                action_type = excluded.action_type,
                action_params = excluded.action_params,
                enabled = excluded.enabled,
                max_fires = excluded.max_fires,
                cooldown_ms = excluded.cooldown_ms",
            rusqlite::params![
                rule.id,
                rule.name,
                trigger_json,
                condition_json,
                rule.action_type,
                action_params,
                rule.enabled as i32,
                rule.max_fires.map(|v| v as i64),
                rule.cooldown_ms.map(|v| v as i64),
            ],
        )?;
        Ok(())
    }

    /// Load all rules from the database.
    /// Load all rules from the database, reconstructing Rule structs.
    pub fn load_rules(&self) -> anyhow::Result<Vec<crate::protocol::Rule>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, trigger_json, condition_json, action_type, action_params, enabled, max_fires, cooldown_ms FROM rules",
        )?;
        let rows = stmt
            .query_map([], |row| {
                let action_params_str: String = row.get::<_, String>(5).unwrap_or_default();
                let action_params: serde_json::Value =
                    serde_json::from_str(&action_params_str).unwrap_or_default();
                Ok(crate::protocol::Rule {
                    id: row.get::<_, String>(0)?,
                    name: row.get::<_, String>(1)?,
                    trigger: serde_json::from_str(&row.get::<_, String>(2)?)
                        .unwrap_or(EventTrigger::ClipboardChanged),
                    condition: row
                        .get::<_, Option<String>>(3)?
                        .and_then(|s| serde_json::from_str(&s).ok()),
                    action_type: row.get::<_, String>(4)?,
                    action_params,
                    enabled: row.get::<_, bool>(6)?,
                    max_fires: row.get::<_, Option<i64>>(7)?.map(|v| v as u32),
                    cooldown_ms: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Delete a rule by ID. Returns true if a row was removed.
    pub fn delete_rule(&self, rule_id: &str) -> anyhow::Result<bool> {
        let affected = self.conn.execute(
            "DELETE FROM rules WHERE id = ?1",
            rusqlite::params![rule_id],
        )?;
        Ok(affected > 0)
    }

    /// Enable or disable a rule.
    pub fn set_rule_enabled(&self, rule_id: &str, enabled: bool) -> anyhow::Result<bool> {
        let affected = self.conn.execute(
            "UPDATE rules SET enabled = ?1 WHERE id = ?2",
            rusqlite::params![enabled as i32, rule_id],
        )?;
        Ok(affected > 0)
    }
}

// ── Helpers ──────────────────────────────────────────────

/// Current Unix timestamp in seconds.
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Intermediate row type for clipboard queries.
struct CbRow {
    id: i64,
    text: String,
    source: Option<String>,
    copied_at: i64,
}

/// Intermediate row type for audit queries.
struct AuditRow {
    id: i64,
    seq: i64,
    uid: i64,
    action: String,
    params: Option<String>,
    status: String,
    duration_ms: Option<i64>,
    timestamp: i64,
}

/// Serialize the optional fields of an AuditEntry into a JSON params string.
fn audit_params_json(entry: &AuditEntry) -> Option<String> {
    let has_error = entry.error.is_some();
    let has_dry_run = entry.dry_run.is_some();
    if !has_error && !has_dry_run {
        return None;
    }
    let mut map = serde_json::Map::new();
    if let Some(ref e) = entry.error {
        map.insert("error".to_string(), serde_json::Value::String(e.clone()));
    }
    if let Some(d) = entry.dry_run {
        map.insert("dry_run".to_string(), serde_json::Value::Bool(d));
    }
    Some(serde_json::Value::Object(map).to_string())
}

/// Parse audit params JSON back into error and dry_run fields.
fn parse_audit_params(params: &Option<String>) -> (Option<String>, Option<bool>) {
    let Some(json) = params else {
        return (None, None);
    };
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json) else {
        return (None, None);
    };
    let error = val
        .get("error")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let dry_run = val.get("dry_run").and_then(|v| v.as_bool());
    (error, dry_run)
}
