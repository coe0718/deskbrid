use crate::backend::DesktopBackend;
use crate::protocol::{Action, DeskbridEvent, Region};
use crate::{DaemonState, visual};
use anyhow::Context;
use dashmap::DashMap;
use serde::Serialize;
use serde_json::json;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;
use tracing::warn;

type BackendCell = Arc<RwLock<Option<Box<dyn DesktopBackend>>>>;

const DEFAULT_INTERVAL_MS: u64 = 500;
const MIN_INTERVAL_MS: u64 = 100;
const MAX_INTERVAL_MS: u64 = 10_000;
const DEFAULT_CHANGE_THRESHOLD_PCT: f64 = 1.0;
const DEFAULT_STABLE_DURATION_MS: u64 = 1_000;
const DEFAULT_TEXT_HISTORY: u32 = 20;
const MAX_TEXT_HISTORY: u32 = 100;

#[derive(Default)]
pub(crate) struct WatchRegistry {
    region: DashMap<String, RegionWatchRuntime>,
    text: DashMap<String, TextWatchRuntime>,
}

struct RegionWatchRuntime {
    config: RegionWatchConfig,
    status: Arc<Mutex<RegionWatchStatus>>,
    task: JoinHandle<()>,
}

struct TextWatchRuntime {
    config: TextWatchConfig,
    status: Arc<Mutex<TextWatchStatus>>,
    task: JoinHandle<()>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct RegionWatchConfig {
    pub(crate) name: String,
    pub(crate) monitor: Option<u32>,
    pub(crate) region: Region,
    pub(crate) interval_ms: u64,
    pub(crate) change_threshold_pct: f64,
    pub(crate) notify_on_change: bool,
    pub(crate) notify_on_stable: bool,
    pub(crate) stable_duration_ms: u64,
    pub(crate) auto_save: Option<String>,
    pub(crate) max_changes: Option<u32>,
    pub(crate) tolerance: u8,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct TextWatchConfig {
    pub(crate) name: String,
    pub(crate) monitor: Option<u32>,
    pub(crate) region: Region,
    pub(crate) interval_ms: u64,
    pub(crate) language: Option<String>,
    pub(crate) notify_on_change: bool,
    pub(crate) notify_on_match: Option<String>,
    pub(crate) notify_on_mismatch: Option<String>,
    pub(crate) max_entries: u32,
    pub(crate) psm: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct RegionWatchStatus {
    changes_seen: u32,
    last_changed: Option<u64>,
    last_stable: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct TextWatchStatus {
    last_text: Option<String>,
    history: VecDeque<TextHistoryEntry>,
}

#[derive(Debug, Clone, Serialize)]
struct TextHistoryEntry {
    timestamp: u64,
    text: String,
}

impl WatchRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn list_region(&self) -> serde_json::Value {
        let watches = self
            .region
            .iter()
            .map(|entry| {
                let status = entry
                    .status
                    .lock()
                    .map(|status| status.clone())
                    .unwrap_or_default();
                json!({
                    "config": entry.config,
                    "status": status
                })
            })
            .collect::<Vec<_>>();
        json!({"watches": watches, "count": watches.len()})
    }

    pub(crate) fn list_text(&self) -> serde_json::Value {
        let watches = self
            .text
            .iter()
            .map(|entry| {
                let status = entry
                    .status
                    .lock()
                    .map(|status| status.clone())
                    .unwrap_or_default();
                json!({
                    "config": entry.config,
                    "status": status
                })
            })
            .collect::<Vec<_>>();
        json!({"watches": watches, "count": watches.len()})
    }

    pub(crate) async fn create_region(
        self: Arc<Self>,
        config: RegionWatchConfig,
        backend: BackendCell,
        event_tx: broadcast::Sender<DeskbridEvent>,
    ) -> anyhow::Result<()> {
        if self.region.contains_key(&config.name) {
            anyhow::bail!("region watch '{}' already exists", config.name);
        }
        let status = Arc::new(Mutex::new(RegionWatchStatus::default()));
        let task = tokio::spawn(region_watch_loop(
            Arc::clone(&self),
            config.clone(),
            Arc::clone(&status),
            backend,
            event_tx,
        ));
        self.region.insert(
            config.name.clone(),
            RegionWatchRuntime {
                config,
                status,
                task,
            },
        );
        Ok(())
    }

    pub(crate) async fn update_region(
        self: Arc<Self>,
        update: RegionWatchUpdate,
        backend: BackendCell,
        event_tx: broadcast::Sender<DeskbridEvent>,
    ) -> anyhow::Result<RegionWatchConfig> {
        let existing = self
            .region
            .get(&update.name)
            .with_context(|| format!("region watch '{}' not found", update.name))?
            .config
            .clone();
        let config = update.apply(existing);
        self.remove_region(&config.name)?;
        Arc::clone(&self)
            .create_region(config.clone(), backend, event_tx)
            .await?;
        Ok(config)
    }

    pub(crate) fn remove_region(&self, name: &str) -> anyhow::Result<()> {
        let (_, runtime) = self
            .region
            .remove(name)
            .with_context(|| format!("region watch '{name}' not found"))?;
        runtime.task.abort();
        Ok(())
    }

    pub(crate) async fn create_text(
        self: Arc<Self>,
        config: TextWatchConfig,
        backend: BackendCell,
        event_tx: broadcast::Sender<DeskbridEvent>,
    ) -> anyhow::Result<()> {
        if self.text.contains_key(&config.name) {
            anyhow::bail!("text watch '{}' already exists", config.name);
        }
        let status = Arc::new(Mutex::new(TextWatchStatus::default()));
        let task = tokio::spawn(text_watch_loop(
            Arc::clone(&self),
            config.clone(),
            Arc::clone(&status),
            backend,
            event_tx,
        ));
        self.text.insert(
            config.name.clone(),
            TextWatchRuntime {
                config,
                status,
                task,
            },
        );
        Ok(())
    }

    pub(crate) fn remove_text(&self, name: &str) -> anyhow::Result<()> {
        let (_, runtime) = self
            .text
            .remove(name)
            .with_context(|| format!("text watch '{name}' not found"))?;
        runtime.task.abort();
        Ok(())
    }
}

pub(crate) struct RegionWatchUpdate {
    name: String,
    monitor: Option<u32>,
    region: Option<Region>,
    interval_ms: Option<u64>,
    change_threshold_pct: Option<f64>,
    notify_on_change: Option<bool>,
    notify_on_stable: Option<bool>,
    stable_duration_ms: Option<u64>,
    auto_save: Option<String>,
    max_changes: Option<u32>,
    tolerance: Option<u8>,
}

impl RegionWatchUpdate {
    fn apply(self, mut config: RegionWatchConfig) -> RegionWatchConfig {
        config.monitor = self.monitor.or(config.monitor);
        if let Some(region) = self.region {
            config.region = region;
        }
        if let Some(interval_ms) = self.interval_ms {
            config.interval_ms = normalize_interval(interval_ms);
        }
        if let Some(change_threshold_pct) = self.change_threshold_pct {
            config.change_threshold_pct = change_threshold_pct;
        }
        if let Some(notify_on_change) = self.notify_on_change {
            config.notify_on_change = notify_on_change;
        }
        if let Some(notify_on_stable) = self.notify_on_stable {
            config.notify_on_stable = notify_on_stable;
        }
        if let Some(stable_duration_ms) = self.stable_duration_ms {
            config.stable_duration_ms = stable_duration_ms;
        }
        if let Some(auto_save) = self.auto_save {
            config.auto_save = Some(auto_save);
        }
        if let Some(max_changes) = self.max_changes {
            config.max_changes = Some(max_changes);
        }
        if let Some(tolerance) = self.tolerance {
            config.tolerance = tolerance;
        }
        config
    }
}

pub(crate) async fn execute_watch_action(
    action: Action,
    state: &DaemonState,
) -> anyhow::Result<serde_json::Value> {
    match action {
        Action::RegionWatchCreate {
            name,
            monitor,
            region,
            interval_ms,
            change_threshold_pct,
            notify_on_change,
            notify_on_stable,
            stable_duration_ms,
            auto_save,
            max_changes,
            tolerance,
        } => {
            let config = RegionWatchConfig {
                name: name.clone(),
                monitor,
                region,
                interval_ms: normalize_interval(interval_ms.unwrap_or(DEFAULT_INTERVAL_MS)),
                change_threshold_pct: change_threshold_pct.unwrap_or(DEFAULT_CHANGE_THRESHOLD_PCT),
                notify_on_change,
                notify_on_stable,
                stable_duration_ms: stable_duration_ms.unwrap_or(DEFAULT_STABLE_DURATION_MS),
                auto_save,
                max_changes,
                tolerance: tolerance.unwrap_or(0),
            };
            Arc::clone(&state.watchers)
                .create_region(
                    config.clone(),
                    Arc::clone(&state.backend),
                    state.event_tx.clone(),
                )
                .await?;
            Ok(json!({"created": name, "watch": config}))
        }
        Action::RegionWatchUpdate {
            name,
            monitor,
            region,
            interval_ms,
            change_threshold_pct,
            notify_on_change,
            notify_on_stable,
            stable_duration_ms,
            auto_save,
            max_changes,
            tolerance,
        } => {
            let config = Arc::clone(&state.watchers)
                .update_region(
                    RegionWatchUpdate {
                        name: name.clone(),
                        monitor,
                        region,
                        interval_ms,
                        change_threshold_pct,
                        notify_on_change,
                        notify_on_stable,
                        stable_duration_ms,
                        auto_save,
                        max_changes,
                        tolerance,
                    },
                    Arc::clone(&state.backend),
                    state.event_tx.clone(),
                )
                .await?;
            Ok(json!({"updated": name, "watch": config}))
        }
        Action::RegionWatchRemove { name } => {
            state.watchers.remove_region(&name)?;
            Ok(json!({"removed": name}))
        }
        Action::RegionWatchList => Ok(state.watchers.list_region()),
        Action::TextWatchCreate {
            name,
            monitor,
            region,
            interval_ms,
            language,
            notify_on_change,
            notify_on_match,
            notify_on_mismatch,
            max_entries,
            psm,
        } => {
            let config = TextWatchConfig {
                name: name.clone(),
                monitor,
                region,
                interval_ms: normalize_interval(interval_ms.unwrap_or(DEFAULT_INTERVAL_MS)),
                language,
                notify_on_change,
                notify_on_match,
                notify_on_mismatch,
                max_entries: max_entries
                    .unwrap_or(DEFAULT_TEXT_HISTORY)
                    .clamp(1, MAX_TEXT_HISTORY),
                psm,
            };
            Arc::clone(&state.watchers)
                .create_text(
                    config.clone(),
                    Arc::clone(&state.backend),
                    state.event_tx.clone(),
                )
                .await?;
            Ok(json!({"created": name, "watch": config}))
        }
        Action::TextWatchRemove { name } => {
            state.watchers.remove_text(&name)?;
            Ok(json!({"removed": name}))
        }
        Action::TextWatchList => Ok(state.watchers.list_text()),
        _ => anyhow::bail!("internal dispatch error: not a watch action"),
    }
}

fn normalize_interval(interval_ms: u64) -> u64 {
    interval_ms.clamp(MIN_INTERVAL_MS, MAX_INTERVAL_MS)
}

async fn region_watch_loop(
    registry: Arc<WatchRegistry>,
    config: RegionWatchConfig,
    status: Arc<Mutex<RegionWatchStatus>>,
    backend: BackendCell,
    event_tx: broadcast::Sender<DeskbridEvent>,
) {
    let mut previous_path: Option<PathBuf> = None;
    let mut stable_since: Option<Instant> = None;
    let mut stable_sent = false;

    loop {
        tokio::time::sleep(Duration::from_millis(config.interval_ms)).await;
        let current_path =
            match capture_region(&backend, config.monitor, config.region.clone()).await {
                Ok(path) => path,
                Err(error) => {
                    warn!("region watch '{}' capture failed: {error}", config.name);
                    continue;
                }
            };

        let Some(before_path) = previous_path.take() else {
            previous_path = Some(current_path);
            continue;
        };

        let stats = match visual::diff_image_paths(
            before_path.clone(),
            current_path.clone(),
            config.tolerance,
        )
        .await
        {
            Ok(stats) => stats,
            Err(error) => {
                warn!("region watch '{}' diff failed: {error}", config.name);
                let _ = tokio::fs::remove_file(before_path).await;
                let _ = tokio::fs::remove_file(&current_path).await;
                continue;
            }
        };

        let changed_pct = percent_changed(&stats);
        if stats.changed_pixels > 0 && changed_pct >= config.change_threshold_pct {
            stable_since = None;
            stable_sent = false;
            let changes_seen = update_region_status(&status, true);
            let screenshot_path =
                save_frame(config.auto_save.as_deref(), &config.name, &current_path)
                    .await
                    .ok()
                    .flatten();
            if config.notify_on_change {
                let _ = event_tx.send(DeskbridEvent::RegionChanged {
                    name: config.name.clone(),
                    changed_pct,
                    bounding_boxes: stats.bbox.map(region_from_bbox).into_iter().collect(),
                    screenshot_path,
                    timestamp: unix_now(),
                });
            }
            if config
                .max_changes
                .is_some_and(|max_changes| changes_seen >= max_changes)
            {
                let _ = tokio::fs::remove_file(before_path).await;
                let _ = tokio::fs::remove_file(&current_path).await;
                registry.region.remove(&config.name);
                break;
            }
        } else if config.notify_on_stable {
            let now = Instant::now();
            let since = stable_since.get_or_insert(now);
            let duration_ms = since.elapsed().as_millis() as u64;
            if !stable_sent && duration_ms >= config.stable_duration_ms {
                stable_sent = true;
                update_region_status(&status, false);
                let screenshot_path =
                    save_frame(config.auto_save.as_deref(), &config.name, &current_path)
                        .await
                        .ok()
                        .flatten();
                let _ = event_tx.send(DeskbridEvent::RegionStable {
                    name: config.name.clone(),
                    duration_ms,
                    screenshot_path,
                    timestamp: unix_now(),
                });
            }
        }

        let _ = tokio::fs::remove_file(before_path).await;
        previous_path = Some(current_path);
    }
}

async fn text_watch_loop(
    _registry: Arc<WatchRegistry>,
    config: TextWatchConfig,
    status: Arc<Mutex<TextWatchStatus>>,
    backend: BackendCell,
    event_tx: broadcast::Sender<DeskbridEvent>,
) {
    let mut last_text: Option<String> = None;
    let mut last_match_state: Option<bool> = None;
    let mut last_mismatch_state: Option<bool> = None;

    loop {
        tokio::time::sleep(Duration::from_millis(config.interval_ms)).await;
        let path = match capture_region(&backend, config.monitor, config.region.clone()).await {
            Ok(path) => path,
            Err(error) => {
                warn!("text watch '{}' capture failed: {error}", config.name);
                continue;
            }
        };
        let text = match extract_text_from_path(&backend, &config, &path).await {
            Ok(text) => text,
            Err(error) => {
                warn!("text watch '{}' OCR failed: {error}", config.name);
                let _ = tokio::fs::remove_file(path).await;
                continue;
            }
        };
        let _ = tokio::fs::remove_file(path).await;

        if config.notify_on_change && last_text.as_ref().is_some_and(|old| old != &text) {
            let _ = event_tx.send(DeskbridEvent::TextChanged {
                name: config.name.clone(),
                old_text: last_text.clone(),
                new_text: text.clone(),
                region: config.region.clone(),
                timestamp: unix_now(),
            });
        }

        if let Some(pattern) = &config.notify_on_match {
            let matched = text.contains(pattern);
            if matched && last_match_state != Some(true) {
                let _ = event_tx.send(DeskbridEvent::TextMatched {
                    name: config.name.clone(),
                    text: text.clone(),
                    pattern: pattern.clone(),
                    region: config.region.clone(),
                    timestamp: unix_now(),
                });
            }
            last_match_state = Some(matched);
        }

        if let Some(pattern) = &config.notify_on_mismatch {
            let matched = text.contains(pattern);
            if last_mismatch_state == Some(true) && !matched {
                let _ = event_tx.send(DeskbridEvent::TextMismatched {
                    name: config.name.clone(),
                    text: text.clone(),
                    pattern: pattern.clone(),
                    region: config.region.clone(),
                    timestamp: unix_now(),
                });
            }
            last_mismatch_state = Some(matched);
        }

        update_text_status(&status, text.clone(), config.max_entries);
        last_text = Some(text);
    }
}

async fn capture_region(
    backend: &BackendCell,
    monitor: Option<u32>,
    region: Region,
) -> anyhow::Result<PathBuf> {
    let backend_guard = backend.read().await;
    let backend = backend_guard
        .as_ref()
        .context("no desktop backend loaded for watch capture")?;
    let screenshot = backend.screenshot(monitor, Some(region), None).await?;
    Ok(PathBuf::from(screenshot.path))
}

async fn extract_text_from_path(
    backend: &BackendCell,
    config: &TextWatchConfig,
    path: &Path,
) -> anyhow::Result<String> {
    let backend_guard = backend.read().await;
    let backend = backend_guard
        .as_ref()
        .context("no desktop backend loaded for OCR")?;
    let path_string = path.to_string_lossy().to_string();
    let result = crate::ocr::screenshot_ocr(
        backend.as_ref(),
        crate::ocr::OcrRequest {
            path: Some(&path_string),
            language: config.language.as_deref(),
            psm: config.psm,
            bounding_boxes: false,
            monitor: None,
            region: None,
            window_id: None,
        },
    )
    .await?;
    Ok(result["text"].as_str().unwrap_or_default().to_string())
}

async fn save_frame(
    auto_save: Option<&str>,
    name: &str,
    source_path: &Path,
) -> anyhow::Result<Option<String>> {
    let Some(auto_save) = auto_save else {
        return Ok(None);
    };
    let dir = crate::daemon::expand_path(auto_save)?;
    tokio::fs::create_dir_all(&dir).await?;
    let target = dir.join(format!("{}-{}.png", safe_name(name), unix_now_millis()));
    tokio::fs::copy(source_path, &target).await?;
    Ok(Some(target.to_string_lossy().to_string()))
}

fn update_region_status(status: &Arc<Mutex<RegionWatchStatus>>, changed: bool) -> u32 {
    let Ok(mut status) = status.lock() else {
        return 0;
    };
    let now = unix_now();
    if changed {
        status.changes_seen = status.changes_seen.saturating_add(1);
        status.last_changed = Some(now);
    } else {
        status.last_stable = Some(now);
    }
    status.changes_seen
}

fn update_text_status(status: &Arc<Mutex<TextWatchStatus>>, text: String, max_entries: u32) {
    let Ok(mut status) = status.lock() else {
        return;
    };
    status.last_text = Some(text.clone());
    status.history.push_back(TextHistoryEntry {
        timestamp: unix_now(),
        text,
    });
    while status.history.len() > max_entries as usize {
        status.history.pop_front();
    }
}

fn percent_changed(stats: &visual::DiffStats) -> f64 {
    if stats.total_pixels == 0 {
        0.0
    } else {
        (stats.changed_pixels as f64 / stats.total_pixels as f64 * 100_000.0).round() / 1000.0
    }
}

fn region_from_bbox(bbox: visual::BoundingBox) -> Region {
    Region {
        x: bbox.x,
        y: bbox.y,
        width: bbox.width,
        height: bbox.height,
    }
}

fn safe_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn unix_now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_is_clamped() {
        assert_eq!(normalize_interval(1), MIN_INTERVAL_MS);
        assert_eq!(normalize_interval(250), 250);
        assert_eq!(normalize_interval(60_000), MAX_INTERVAL_MS);
    }

    #[test]
    fn percent_changed_rounds_to_three_decimals() {
        let stats = visual::DiffStats {
            width: 10,
            height: 10,
            total_pixels: 100,
            changed_pixels: 12,
            bbox: None,
        };
        assert_eq!(percent_changed(&stats), 12.0);
    }
}
