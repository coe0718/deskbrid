use crate::protocol::AppCatalogEntry;
use std::path::Path;

pub(crate) async fn collect_desktop_entries(
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

pub(crate) fn parse_desktop_entry(path: &Path, raw: &str) -> Option<AppCatalogEntry> {
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

pub(crate) fn split_list(value: Option<&String>) -> Vec<String> {
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

pub(crate) fn parse_bool(value: &str) -> bool {
    matches!(value.trim().to_lowercase().as_str(), "true" | "1" | "yes")
}
