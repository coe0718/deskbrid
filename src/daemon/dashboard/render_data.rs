use crate::DaemonState;

use super::html_escape;

// ── State-backed card renderers ──────────────────────────

async fn execute_secrets_collections(state: &DaemonState) -> anyhow::Result<String> {
    let result = crate::daemon::execute_secrets::execute_secrets_action(
        crate::protocol::Action::SecretsListCollections,
        state,
    )
    .await?;
    let collections = result["collections"].as_array();
    match collections {
        Some(list) if !list.is_empty() => {
            let mut rows = String::new();
            for col in list.iter().take(10) {
                let path = col["path"].as_str().unwrap_or("?");
                let label = col["label"].as_str().unwrap_or(path);
                let locked = col["locked"].as_bool();
                let lock_icon = match locked {
                    Some(true) => "🔒",
                    Some(false) => "🔓",
                    None => "❓",
                };
                rows.push_str(&super::kv(&format!("{} {}", lock_icon, label), path));
            }
            if list.len() > 10 {
                rows.push_str(&format!(
                    r#"<div class="empty">… and {} more</div>"#,
                    list.len() - 10
                ));
            }
            Ok(rows)
        }
        _ => Ok(r#"<div class="empty">No collections found</div>"#.into()),
    }
}

pub(super) async fn render_clipboard(state: &DaemonState) -> String {
    let db = state.database.lock().await;
    match db.get_clipboard_history(10, None) {
        Ok(entries) => {
            if entries.is_empty() {
                return r#"<div class="empty">No clipboard history</div>"#.into();
            }
            let mut rows = String::new();
            for e in entries.iter().rev() {
                let text = if e.text.len() > 80 {
                    format!("{}…", &e.text[..80])
                } else {
                    e.text.clone()
                };
                rows.push_str(&format!(
                    r#"<div class="clip-entry"><span class="clip-text">{}</span><span class="clip-src">{}</span><span class="clip-ts">#{}</span></div>"#,
                    html_escape(&text),
                    html_escape(&e.source),
                    e.id,
                ));
            }
            rows
        }
        Err(_) => r#"<div class="empty">DB unavailable</div>"#.into(),
    }
}

pub(super) async fn render_audit(state: &DaemonState) -> String {
    let entries = state.audit_log.lock().await;
    if entries.is_empty() {
        return r#"<div class="empty">No audit entries yet</div>"#.into();
    }
    let count = entries.len();
    let show = count.min(30);
    let start = count - show;
    let mut rows = String::new();
    for (i, entry) in entries.iter().enumerate() {
        if i < start {
            continue;
        }
        let sc = if entry.status == "ok" { "ok" } else { "error" };
        let dur = if entry.duration_ms >= 1000 {
            format!("{:.1}s", entry.duration_ms as f64 / 1000.0)
        } else {
            format!("{}ms", entry.duration_ms)
        };
        let mut ad = entry.action_type.clone();
        if let Some(ref err) = entry.error {
            ad.push_str(&format!(" — {}", err));
        }
        rows.push_str(&format!(
            r#"<div class="audit-row"><span class="audit-status {sc}">{st}</span><span class="audit-action">{ac}</span><span style="color:#64748b;font-size:0.65rem">uid:{uid} {dur}</span><span class="audit-ts">#{id}</span></div>"#,
            sc = sc,
            st = entry.status,
            ac = html_escape(&ad),
            uid = entry.peer_uid,
            dur = dur,
            id = entry.id,
        ));
    }
    rows
}

pub(super) async fn render_sessions(state: &DaemonState) -> String {
    let sessions = state.sessions.lock().await;
    if sessions.is_empty() {
        return r#"<div class="empty">No sessions</div>"#.into();
    }
    let mut rows = String::new();
    for s in sessions.values() {
        rows.push_str(&super::kv(&s.name, &format!("{} vars", s.vars.len())));
    }
    rows
}

pub(super) async fn render_rules(state: &DaemonState) -> String {
    let rules = state.rules.lock().await;
    let list = rules.list();
    if list.is_empty() {
        return r#"<div class="empty">No rules configured</div>"#.into();
    }
    let mut rows = String::new();
    for r in list.iter().take(10) {
        let enabled = if r.enabled {
            r#"<span class="rule-enabled">ON</span>"#
        } else {
            r#"<span class="rule-disabled">OFF</span>"#
        };
        rows.push_str(&format!(
            r#"<div class="rule-row"><span class="rule-name">{}</span><span class="rule-trigger">{:?}</span>{}</div>"#,
            html_escape(&r.name),
            r.trigger,
            enabled,
        ));
    }
    rows
}

pub(super) async fn render_notifications(state: &DaemonState) -> String {
    let db = state.database.lock().await;
    match db.get_notifications(8, None, None) {
        Ok(entries) => {
            if entries.is_empty() {
                return r#"<div class="empty">No notifications</div>"#.into();
            }
            let mut rows = String::new();
            for n in entries.iter().rev() {
                let app = n["app_name"].as_str().unwrap_or("?");
                let title = n["title"].as_str().unwrap_or("(no title)");
                let id = n["id"].as_u64().unwrap_or(0);
                rows.push_str(&format!(
                    r#"<div class="audit-row"><span style="color:#64748b;font-size:0.65rem">{}</span><span style="flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:0.75rem">{}</span><span class="audit-ts">#{}</span></div>"#,
                    html_escape(app),
                    html_escape(title),
                    id,
                ));
            }
            rows
        }
        Err(_) => r#"<div class="empty">DB unavailable</div>"#.into(),
    }
}

pub(super) async fn render_macros() -> String {
    match crate::daemon::macro_engine::list_macros().await {
        Ok(list) => {
            if list.is_empty() {
                return r#"<div class="empty">No macros recorded</div>"#.into();
            }
            let mut rows = String::new();
            for m in list.iter().take(8) {
                rows.push_str(&super::kv(&m.name, &format!("{} actions", m.action_count)));
            }
            rows
        }
        Err(_) => r#"<div class="empty">Macro engine unavailable</div>"#.into(),
    }
}

pub(super) async fn render_confirmations(state: &DaemonState) -> String {
    let pending = state.pending_confirmations.lock().await;
    if pending.is_empty() {
        return r#"<div class="empty">No pending confirmations</div>"#.into();
    }
    let mut rows = String::new();
    for (id, entry) in pending.iter().take(10) {
        let truncated_id: String = id.chars().take(12).collect();
        rows.push_str(&super::kv(
            &format!("⚠ {}", entry.action.action_type()),
            &truncated_id,
        ));
    }
    if pending.len() > 10 {
        rows.push_str(&format!(
            r#"<div class="empty">… and {} more</div>"#,
            pending.len() - 10
        ));
    }
    rows
}

pub(super) async fn render_secrets(state: &DaemonState) -> String {
    match execute_secrets_collections(state).await {
        Ok(html) => html,
        Err(_) => r#"<div class="empty">Secrets DB unavailable</div>"#.into(),
    }
}

pub(super) async fn render_agent_mailbox(state: &DaemonState) -> String {
    let messages = state.agent_mailbox.get_for("dashboard").await;
    if messages.is_empty() {
        return r#"<div class="empty">No messages</div>"#.into();
    }
    let mut rows = String::new();
    for m in messages.iter().rev().take(8) {
        let prefix = if m.to_session.is_some() {
            "📩"
        } else {
            "📢"
        };
        let body_preview: String = m.body.as_str().unwrap_or("?").chars().take(40).collect();
        rows.push_str(&super::kv(
            &format!("{} {}", prefix, m.subject),
            &format!("#{} {}", m.id, body_preview),
        ));
    }
    rows
}

pub(super) async fn render_search(state: &DaemonState) -> String {
    let stats = state.search_index.stats().await;
    let surfaces: Vec<String> = stats["surfaces"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let max_results = stats["max_results"].as_u64().unwrap_or(0);
    let mut rows = super::kv(
        "Surfaces",
        &format!("{} ({})", surfaces.join(", "), surfaces.len()),
    );
    rows.push_str(&super::kv("Max Results", &max_results.to_string()));
    rows
}
