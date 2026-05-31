use anyhow::Context;
use rusqlite::params;

use super::Database;

impl Database {
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
}
