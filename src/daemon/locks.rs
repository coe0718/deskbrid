use crate::daemon::agent_registry::{now_ms, now_secs};
use crate::protocol::{Action, DeskbridEvent};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tokio::sync::{Mutex, broadcast};

const DEFAULT_LOCK_TTL_MS: u64 = 30_000;
const LOCK_WAIT_POLL_MS: u64 = 50;

#[derive(Debug, Clone, Serialize)]
pub struct LockEntry {
    pub resource: String,
    pub holder: String,
    pub token: String,
    pub acquired_at: u64,
    pub expires_at: u64,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockAcquireResult {
    pub acquired: bool,
    pub lock: Option<LockEntry>,
    pub owner: Option<LockEntry>,
    pub timed_out: bool,
    pub already_held: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockReleaseResult {
    pub released: bool,
    pub lock: Option<LockEntry>,
    pub reason: Option<String>,
}

#[derive(Default)]
pub struct LockStore {
    locks: Mutex<HashMap<String, LockEntry>>,
}

impl LockStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn acquire(
        &self,
        resource: String,
        holder: String,
        ttl_ms: Option<u64>,
        wait_ms: Option<u64>,
        force: bool,
        event_tx: &broadcast::Sender<DeskbridEvent>,
    ) -> LockAcquireResult {
        let ttl_ms = ttl_ms.unwrap_or(DEFAULT_LOCK_TTL_MS).max(1);
        let wait_ms = wait_ms.unwrap_or(0);
        let started = std::time::Instant::now();

        loop {
            let mut events = Vec::new();
            let mut should_wait_on: Option<LockEntry> = None;
            let result = {
                let mut locks = self.locks.lock().await;
                prune_expired_locked(&mut locks, &mut events);

                match locks.get(&resource).cloned() {
                    Some(existing) if existing.holder == holder => Some(LockAcquireResult {
                        acquired: true,
                        lock: Some(existing),
                        owner: None,
                        timed_out: false,
                        already_held: true,
                    }),
                    Some(existing) if force => {
                        locks.remove(&resource);
                        let entry = new_lock_entry(resource.clone(), holder.clone(), ttl_ms);
                        events.push(DeskbridEvent::LockStolen {
                            resource: resource.clone(),
                            previous_holder: existing.holder.clone(),
                            new_holder: holder.clone(),
                            token: entry.token.clone(),
                            timestamp: now_secs(),
                        });
                        events.push(DeskbridEvent::LockAcquired {
                            resource: resource.clone(),
                            holder: holder.clone(),
                            token: entry.token.clone(),
                            expires_at: entry.expires_at,
                            timestamp: now_secs(),
                        });
                        locks.insert(resource.clone(), entry.clone());
                        Some(LockAcquireResult {
                            acquired: true,
                            lock: Some(entry),
                            owner: Some(existing),
                            timed_out: false,
                            already_held: false,
                        })
                    }
                    Some(existing) => {
                        should_wait_on = Some(existing);
                        None
                    }
                    None => {
                        let entry = new_lock_entry(resource.clone(), holder.clone(), ttl_ms);
                        events.push(DeskbridEvent::LockAcquired {
                            resource: resource.clone(),
                            holder: holder.clone(),
                            token: entry.token.clone(),
                            expires_at: entry.expires_at,
                            timestamp: now_secs(),
                        });
                        locks.insert(resource.clone(), entry.clone());
                        Some(LockAcquireResult {
                            acquired: true,
                            lock: Some(entry),
                            owner: None,
                            timed_out: false,
                            already_held: false,
                        })
                    }
                }
            };

            emit_events(event_tx, events);

            if let Some(result) = result {
                return result;
            }

            let owner = should_wait_on.expect("wait target must be set for contested lock");
            if started.elapsed().as_millis() as u64 >= wait_ms {
                let _ = event_tx.send(DeskbridEvent::LockTimeout {
                    resource,
                    holder,
                    owner: Some(owner.holder.clone()),
                    reason: "wait_timeout".to_string(),
                    timestamp: now_secs(),
                });
                return LockAcquireResult {
                    acquired: false,
                    lock: None,
                    owner: Some(owner),
                    timed_out: true,
                    already_held: false,
                };
            }

            let elapsed = started.elapsed().as_millis() as u64;
            let remaining = wait_ms.saturating_sub(elapsed);
            tokio::time::sleep(std::time::Duration::from_millis(
                LOCK_WAIT_POLL_MS.min(remaining.max(1)),
            ))
            .await;
        }
    }

    pub async fn release(
        &self,
        resource: String,
        token: String,
        event_tx: &broadcast::Sender<DeskbridEvent>,
    ) -> LockReleaseResult {
        let mut events = Vec::new();
        let result = {
            let mut locks = self.locks.lock().await;
            prune_expired_locked(&mut locks, &mut events);
            match locks.get(&resource).cloned() {
                None => LockReleaseResult {
                    released: false,
                    lock: None,
                    reason: Some("not_found".to_string()),
                },
                Some(existing) if existing.token != token => LockReleaseResult {
                    released: false,
                    lock: Some(existing),
                    reason: Some("token_mismatch".to_string()),
                },
                Some(existing) => {
                    locks.remove(&resource);
                    events.push(DeskbridEvent::LockReleased {
                        resource: resource.clone(),
                        holder: existing.holder.clone(),
                        token: existing.token.clone(),
                        reason: "released".to_string(),
                        timestamp: now_secs(),
                    });
                    LockReleaseResult {
                        released: true,
                        lock: Some(existing),
                        reason: None,
                    }
                }
            }
        };
        emit_events(event_tx, events);
        result
    }

    pub async fn release_holders<I>(
        &self,
        holders: I,
        event_tx: &broadcast::Sender<DeskbridEvent>,
    ) -> Vec<LockEntry>
    where
        I: IntoIterator<Item = String>,
    {
        let holders: HashSet<String> = holders.into_iter().collect();
        if holders.is_empty() {
            return Vec::new();
        }

        let mut released = Vec::new();
        let mut events = Vec::new();
        {
            let mut locks = self.locks.lock().await;
            prune_expired_locked(&mut locks, &mut events);
            let resources: Vec<String> = locks
                .iter()
                .filter(|(_, entry)| holders.contains(&entry.holder))
                .map(|(resource, _)| resource.clone())
                .collect();
            for resource in resources {
                if let Some(entry) = locks.remove(&resource) {
                    events.push(DeskbridEvent::LockReleased {
                        resource: entry.resource.clone(),
                        holder: entry.holder.clone(),
                        token: entry.token.clone(),
                        reason: "holder_disconnected".to_string(),
                        timestamp: now_secs(),
                    });
                    released.push(entry);
                }
            }
        }
        emit_events(event_tx, events);
        released
    }

    pub async fn list(&self, event_tx: &broadcast::Sender<DeskbridEvent>) -> Vec<LockEntry> {
        let mut events = Vec::new();
        let mut locks = self.locks.lock().await;
        prune_expired_locked(&mut locks, &mut events);
        let mut entries: Vec<_> = locks.values().cloned().collect();
        entries.sort_by(|a, b| a.resource.cmp(&b.resource));
        drop(locks);
        emit_events(event_tx, events);
        entries
    }

    pub async fn resources_for_holders<I>(&self, holders: I) -> Vec<String>
    where
        I: IntoIterator<Item = String>,
    {
        let holders: HashSet<String> = holders.into_iter().collect();
        let locks = self.locks.lock().await;
        let mut resources: Vec<_> = locks
            .values()
            .filter(|entry| !entry.is_expired(now_ms()))
            .filter(|entry| holders.contains(&entry.holder))
            .map(|entry| entry.resource.clone())
            .collect();
        resources.sort();
        resources.dedup();
        resources
    }

    pub async fn prune_expired(&self, event_tx: &broadcast::Sender<DeskbridEvent>) {
        let mut events = Vec::new();
        {
            let mut locks = self.locks.lock().await;
            prune_expired_locked(&mut locks, &mut events);
        }
        emit_events(event_tx, events);
    }
}

impl LockEntry {
    fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

pub(crate) fn is_lock_action(action: &Action) -> bool {
    matches!(
        action,
        Action::LockAcquire { .. } | Action::LockRelease { .. } | Action::LockList
    )
}

pub(crate) async fn execute_lock_action(
    action: Action,
    state: &crate::DaemonState,
    session_id: &str,
) -> anyhow::Result<serde_json::Value> {
    match action {
        Action::LockAcquire {
            resource,
            holder,
            ttl_ms,
            wait_ms,
            force,
        } => {
            let holder = holder.unwrap_or_else(|| session_id.to_string());
            let result = state
                .locks
                .acquire(resource, holder, ttl_ms, wait_ms, force, &state.event_tx)
                .await;
            Ok(serde_json::to_value(result)?)
        }
        Action::LockRelease { resource, token } => {
            let result = state.locks.release(resource, token, &state.event_tx).await;
            Ok(serde_json::to_value(result)?)
        }
        Action::LockList => {
            let locks = state.locks.list(&state.event_tx).await;
            Ok(serde_json::json!({"locks": locks, "count": locks.len()}))
        }
        _ => anyhow::bail!("internal dispatch error: not a lock action"),
    }
}

pub fn spawn_lock_sweeper(state: std::sync::Arc<crate::DaemonState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            state.locks.prune_expired(&state.event_tx).await;
        }
    });
}

fn new_lock_entry(resource: String, holder: String, ttl_ms: u64) -> LockEntry {
    let acquired_at = now_ms();
    LockEntry {
        resource,
        holder,
        token: uuid::Uuid::new_v4().to_string(),
        acquired_at,
        expires_at: acquired_at.saturating_add(ttl_ms),
        ttl_ms,
    }
}

fn prune_expired_locked(locks: &mut HashMap<String, LockEntry>, events: &mut Vec<DeskbridEvent>) {
    let now = now_ms();
    let expired: Vec<String> = locks
        .iter()
        .filter(|(_, entry)| entry.is_expired(now))
        .map(|(resource, _)| resource.clone())
        .collect();
    for resource in expired {
        if let Some(entry) = locks.remove(&resource) {
            events.push(DeskbridEvent::LockTimeout {
                resource: entry.resource,
                holder: entry.holder,
                owner: None,
                reason: "expired".to_string(),
                timestamp: now_secs(),
            });
        }
    }
}

fn emit_events(event_tx: &broadcast::Sender<DeskbridEvent>, events: Vec<DeskbridEvent>) {
    for event in events {
        let _ = event_tx.send(event);
    }
}
