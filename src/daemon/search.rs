use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct SearchIndex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowHit {
    pub title: String,
    pub app_id: String,
    pub id: String,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppHit {
    pub name: String,
    pub app_id: String,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHit {
    pub path: String,
    pub size: u64,
    pub score: u32,
}

impl Default for SearchIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchIndex {
    pub fn new() -> Self {
        Self
    }

    pub async fn search_files(&self, query: &str, limit: usize) -> anyhow::Result<Vec<FileHit>> {
        let query_lower = query.to_lowercase();
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let mut results = Vec::new();

        for dir in &["Downloads", "Documents", "Desktop", "Pictures"] {
            let path = format!("{}/{}", home, dir);
            if let Ok(mut entries) = tokio::fs::read_dir(&path).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.contains(&query_lower) {
                        let size = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
                        results.push(FileHit {
                            path: format!("{}/{}", dir, entry.file_name().to_string_lossy()),
                            size,
                            score: if name.starts_with(&query_lower) {
                                100
                            } else {
                                50
                            },
                        });
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
            if results.len() >= limit {
                break;
            }
        }
        Ok(results)
    }

    pub async fn stats(&self) -> Value {
        serde_json::json!({
            "type": "in_memory",
            "surfaces": ["windows", "apps", "clipboard", "files", "audit"],
            "filters": ["category", "score"],
            "max_results": 100
        })
    }
}
