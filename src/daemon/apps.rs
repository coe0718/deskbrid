use std::path::PathBuf;

use crate::DaemonState;
use crate::protocol::{Action, AppCatalogEntry};

use super::apps_parse::*;

const DEFAULT_APP_LIMIT: usize = 200;
const MAX_APP_LIMIT: usize = 1000;

pub(crate) fn is_app_catalog_action(action: &Action) -> bool {
    matches!(
        action,
        Action::AppList { .. } | Action::AppSearch { .. } | Action::AppGet { .. }
    )
}

pub(crate) async fn execute_app_catalog_action(
    action: Action,
    _state: &DaemonState,
) -> anyhow::Result<serde_json::Value> {
    match action {
        Action::AppList {
            categories,
            mime_types,
            include_hidden,
            limit,
        } => {
            let entries = load_apps().await?;
            let limit = limit.unwrap_or(DEFAULT_APP_LIMIT).min(MAX_APP_LIMIT);
            let filtered = filter_apps(entries, &categories, &mime_types, include_hidden, limit);
            Ok(serde_json::json!({"apps": filtered, "count": filtered.len()}))
        }
        Action::AppSearch { query, limit } => {
            let query_l = query.to_lowercase();
            let limit = limit.unwrap_or(DEFAULT_APP_LIMIT).min(MAX_APP_LIMIT);
            let apps: Vec<AppCatalogEntry> = load_apps()
                .await?
                .into_iter()
                .filter(|app| !app.no_display)
                .filter(|app| {
                    app.name.to_lowercase().contains(&query_l)
                        || app.app_id.to_lowercase().contains(&query_l)
                        || app
                            .comment
                            .as_deref()
                            .is_some_and(|value| value.to_lowercase().contains(&query_l))
                        || app
                            .categories
                            .iter()
                            .any(|value| value.to_lowercase().contains(&query_l))
                })
                .take(limit)
                .collect();
            Ok(serde_json::json!({"apps": apps, "count": apps.len()}))
        }
        Action::AppGet { app_id } => {
            let app_id_l = app_id.to_lowercase();
            let app = load_apps()
                .await?
                .into_iter()
                .find(|app| {
                    app.app_id.eq_ignore_ascii_case(&app_id)
                        || app
                            .app_id
                            .trim_end_matches(".desktop")
                            .eq_ignore_ascii_case(&app_id_l)
                })
                .ok_or_else(|| anyhow::anyhow!("app not found: {}", app_id))?;
            Ok(serde_json::json!(app))
        }
        _ => anyhow::bail!("not an app catalog action"),
    }
}

fn filter_apps(
    entries: Vec<AppCatalogEntry>,
    categories: &[String],
    mime_types: &[String],
    include_hidden: bool,
    limit: usize,
) -> Vec<AppCatalogEntry> {
    entries
        .into_iter()
        .filter(|app| include_hidden || !app.no_display)
        .filter(|app| {
            categories.is_empty()
                || categories.iter().all(|category| {
                    app.categories
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(category))
                })
        })
        .filter(|app| {
            mime_types.is_empty()
                || mime_types.iter().any(|mime| {
                    app.mime_types
                        .iter()
                        .any(|value| value.eq_ignore_ascii_case(mime))
                })
        })
        .take(limit)
        .collect()
}

async fn load_apps() -> anyhow::Result<Vec<AppCatalogEntry>> {
    let mut entries = Vec::new();
    for dir in app_dirs() {
        collect_desktop_entries(&dir, &mut entries).await?;
    }
    entries.sort_by_key(|a| a.name.to_lowercase());
    let mut seen = std::collections::HashSet::new();
    Ok(entries
        .into_iter()
        .filter(|entry| seen.insert(entry.app_id.clone()))
        .collect())
}

fn app_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(home) = std::env::var("XDG_DATA_HOME") {
        dirs.push(PathBuf::from(home).join("applications"));
    } else if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let data_dirs =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(
        data_dirs
            .split(':')
            .filter(|value| !value.trim().is_empty())
            .map(|value| PathBuf::from(value).join("applications")),
    );
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_desktop_entry() {
        let raw = r#"
[Desktop Entry]
Type=Application
Name=Code
GenericName=Editor
Comment=Write code
Exec=code %F
Icon=code
Categories=Development;IDE;
MimeType=text/plain;inode/directory;
NoDisplay=false
Terminal=false
"#;

        let app = parse_desktop_entry(Path::new("/usr/share/applications/code.desktop"), raw)
            .expect("app");
        assert_eq!(app.app_id, "code.desktop");
        assert_eq!(app.name, "Code");
        assert_eq!(app.categories, vec!["Development", "IDE"]);
        assert_eq!(app.mime_types, vec!["text/plain", "inode/directory"]);
        assert!(!app.no_display);
    }
}
