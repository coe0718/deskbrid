use crate::protocol::Action;
use serde_json::{Value, json};

pub async fn execute_search(
    action: Action,
    state: &crate::DaemonState,
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<Value> {
    match action {
        Action::UnifiedSearch {
            query,
            categories,
            limit,
        } => {
            let limit = limit.unwrap_or(20);
            let cats: Vec<String> = categories.unwrap_or_else(|| {
                vec![
                    "windows".into(),
                    "files".into(),
                    "apps".into(),
                    "clipboard".into(),
                    "audit".into(),
                ]
            });

            let mut results = Vec::new();
            let query_lower = query.to_lowercase();

            for cat in &cats {
                match cat.as_str() {
                    "windows" => {
                        if let Ok(windows) = backend.windows_list().await {
                            let mut scored: Vec<_> = windows
                                .into_iter()
                                .filter_map(|w| {
                                    let title_lower = w.title.to_lowercase();
                                    let app_lower = w.app_id.to_lowercase();
                                    if title_lower.contains(&query_lower)
                                        || app_lower.contains(&query_lower)
                                    {
                                        let score = if title_lower.starts_with(&query_lower) {
                                            100
                                        } else if app_lower.starts_with(&query_lower) {
                                            90
                                        } else {
                                            50
                                        };
                                        Some((score, w))
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
                            for (score, w) in scored.iter().take(limit) {
                                results.push(json!({
                                    "category": "windows",
                                    "title": w.title,
                                    "subtitle": w.app_id,
                                    "source": format!("window:{}", w.id),
                                    "score": score,
                                }));
                            }
                        }
                    }
                    "apps" => {
                        let apps = crate::daemon::apps::load_apps().await?;
                        let mut scored: Vec<_> = apps
                            .into_iter()
                            .filter(|app| !app.no_display)
                            .filter_map(|a| {
                                let name_lower = a.name.to_lowercase();
                                let id_lower = a.app_id.to_lowercase();
                                if name_lower.contains(&query_lower)
                                    || id_lower.contains(&query_lower)
                                {
                                    let score = if name_lower.starts_with(&query_lower) {
                                        100
                                    } else {
                                        60
                                    };
                                    Some((score, a))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
                        for (score, a) in scored.iter().take(limit) {
                            results.push(json!({
                                "category": "apps",
                                "title": a.name,
                                "subtitle": a.app_id,
                                "source": format!("app:{}", a.app_id),
                                "score": score,
                            }));
                        }
                    }
                    "clipboard" => {
                        let history = state.clipboard_history.lock().await;
                        let mut scored: Vec<_> = history
                            .iter()
                            .enumerate()
                            .filter_map(|(i, entry)| {
                                let text_lower = entry.text.to_lowercase();
                                if text_lower.contains(&query_lower) {
                                    let score = if text_lower.starts_with(&query_lower) {
                                        100u32.saturating_sub(i as u32)
                                    } else {
                                        50u32.saturating_sub(i as u32)
                                    };
                                    Some((score, entry))
                                } else {
                                    None
                                }
                            })
                            .collect();
                        scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
                        for (score, entry) in scored.iter().take(limit) {
                            results.push(json!({
                                "category": "clipboard",
                                "title": entry.text.chars().take(80).collect::<String>(),
                                "subtitle": format!("clip #{}", entry.id),
                                "source": format!("clipboard:{}", entry.id),
                                "score": score,
                            }));
                        }
                    }
                    "audit" => {
                        let log = state.audit_log.lock().await;
                        let mut scored: Vec<_> = log
                            .iter()
                            .enumerate()
                            .filter(|(_, entry)| {
                                entry.action_type.to_lowercase().contains(&query_lower)
                                    || entry.status.to_lowercase().contains(&query_lower)
                            })
                            .map(|(i, entry)| {
                                let score = 80u32.saturating_sub(i as u32);
                                (score, entry)
                            })
                            .collect();
                        scored.sort_by_key(|(s, _)| std::cmp::Reverse(*s));
                        for (score, entry) in scored.iter().take(limit) {
                            results.push(json!({
                                "category": "audit",
                                "title": format!("{} — {}", entry.action_type, entry.status),
                                "subtitle": format!("{}ms", entry.duration_ms),
                                "source": format!("audit:{}", entry.seq),
                                "score": score,
                            }));
                        }
                    }
                    "files" => {
                        if let Ok(files) =
                            state.search_index.search_files(&query, limit).await
                        {
                            for f in files {
                                results.push(json!({
                                    "category": "files",
                                    "title": f.path,
                                    "subtitle": format!("{} bytes", f.size),
                                    "source": format!("file:{}", f.path),
                                    "score": f.score,
                                }));
                            }
                        }
                    }
                    _ => {}
                }
            }

            results.sort_by(|a, b| {
                b["score"]
                    .as_u64()
                    .unwrap_or(0)
                    .cmp(&a["score"].as_u64().unwrap_or(0))
            });
            results.truncate(limit);

            Ok(json!({
                "results": results,
                "count": results.len(),
                "query": query,
                "categories_searched": cats,
            }))
        }
        Action::UnifiedIndex => {
            let stats = state.search_index.stats().await;
            Ok(json!({"index": stats}))
        }
        _ => unreachable!("not a search action"),
    }
}
