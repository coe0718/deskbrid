use crate::protocol::AuditEntry;
use anyhow::Context;
use rusqlite::{params, types::ToSql};

use super::Database;

impl Database {
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
        let mut param_values: Vec<Box<dyn ToSql>> = Vec::new();

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

        let params_ref: Vec<&dyn ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

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
}

// ── Helpers ──────────────────────────────────────────────

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
