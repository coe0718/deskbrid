use std::path::{Path, PathBuf};

use crate::DaemonState;
use crate::protocol::{Action, AppCatalogEntry};

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

async fn collect_desktop_entries(
    dir: &Path,
    entries: &mut Vec<AppCatalogEntry>,
) -> anyhow::Result<()> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(mut read_dir) = tokio::fs::read_dir(&path).await else {
            continue;
        };
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("desktop") {
                continue;
            }
            if let Ok(raw) = tokio::fs::read_to_string(&path).await
                && let Some(app) = parse_desktop_entry(&path, &raw)
            {
                entries.push(app);
            }
        }
    }
    Ok(())
}

fn parse_desktop_entry(path: &Path, raw: &str) -> Option<AppCatalogEntry> {
    let mut in_desktop_entry = false;
    let mut values = std::collections::HashMap::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values
            .entry(key.to_string())
            .or_insert_with(|| value.to_string());
    }

    if values
        .get("Type")
        .is_some_and(|value| value != "Application")
    {
        return None;
    }
    if values.get("Hidden").is_some_and(|value| parse_bool(value)) {
        return None;
    }
    let name = values.get("Name")?.to_string();
    let app_id = path.file_name()?.to_string_lossy().to_string();

    Some(AppCatalogEntry {
        app_id,
        name,
        generic_name: values.get("GenericName").cloned(),
        comment: values.get("Comment").cloned(),
        exec: values.get("Exec").cloned(),
        icon: values.get("Icon").cloned(),
        categories: split_list(values.get("Categories")),
        mime_types: split_list(values.get("MimeType")),
        no_display: values
            .get("NoDisplay")
            .is_some_and(|value| parse_bool(value)),
        terminal: values
            .get("Terminal")
            .is_some_and(|value| parse_bool(value)),
        path: path.to_string_lossy().to_string(),
    })
}

fn split_list(value: Option<&String>) -> Vec<String> {
    value
        .map(|value| {
            value
                .split(';')
                .filter(|part| !part.trim().is_empty())
                .map(|part| part.trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_bool(value: &str) -> bool {
    matches!(value.trim().to_lowercase().as_str(), "true" | "1" | "yes")
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
