use super::helpers::*;
use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_files(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        // Files
        "files.watch" => Action::FilesWatch {
            path: required_path(raw, "path")?,
            recursive: raw["recursive"].as_bool().unwrap_or(true),
            patterns: raw["patterns"].as_array().map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            }),
        },
        "files.unwatch" => Action::FilesUnwatch {
            path: required_path(raw, "path")?,
        },
        "files.search" => Action::FilesSearch {
            pattern: raw["pattern"].as_str().unwrap_or("").into(),
            root: raw["root"].as_str().map(String::from),
            max_results: raw["max_results"].as_u64().unwrap_or(50) as u32,
        },
        "files.read" => Action::FilesRead {
            path: required_path(raw, "path")?,
            offset: raw["offset"].as_u64(),
            limit: raw["limit"].as_u64(),
        },
        "files.write" => Action::FilesWrite {
            path: required_path(raw, "path")?,
            content: raw["content"].as_str().unwrap_or("").into(),
            append: raw["append"].as_bool().unwrap_or(false),
        },
        "files.copy" => Action::FilesCopy {
            source: required_path(raw, "source")?,
            destination: required_path(raw, "destination")?,
        },
        "files.move" => Action::FilesMove {
            source: required_path(raw, "source")?,
            destination: required_path(raw, "destination")?,
        },
        "files.delete" => Action::FilesDelete {
            path: required_path(raw, "path")?,
            recursive: raw["recursive"].as_bool().unwrap_or(false),
        },
        "files.mkdir" => Action::FilesMkdir {
            path: required_path(raw, "path")?,
            parents: raw["parents"].as_bool().unwrap_or(true),
        },
        "files.list" => Action::FilesList {
            path: raw["path"].as_str().unwrap_or(".").into(),
        },
        _ => anyhow::bail!("unknown files type: {type_str}"),
    })
}
