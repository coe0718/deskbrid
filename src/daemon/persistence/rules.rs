
use super::Database;

impl Database {
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
                        .unwrap_or(crate::protocol::EventTrigger::ClipboardChanged),
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
