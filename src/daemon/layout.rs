use crate::protocol::{LayoutProfile, LayoutProfileSummary};
use anyhow::Context;
use std::path::PathBuf;

use super::helpers::unix_timestamp;
mod paths;
pub use paths::{layout_profile_path, layout_profiles_dir, validate_layout_profile_name};

pub async fn capture_layout_profile(
    name: &str,
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<LayoutProfile> {
    let name = validate_layout_profile_name(name)?.to_string();
    let info = backend.system_info().await?;
    Ok(LayoutProfile {
        schema_version: 1,
        name,
        saved_at: unix_timestamp(),
        desktop: info.desktop,
        session_type: info.session_type,
        current_workspace: info.current_workspace,
        monitors: info.monitors,
        workspaces: backend.workspaces_list().await?,
        windows: backend.windows_list().await?,
    })
}

pub async fn save_layout_profile(
    profile: &LayoutProfile,
    overwrite: bool,
) -> anyhow::Result<PathBuf> {
    let path = layout_profile_path(&profile.name)?;
    if !overwrite && tokio::fs::metadata(&path).await.is_ok() {
        anyhow::bail!("layout profile '{}' already exists", profile.name);
    }

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let data = serde_json::to_vec_pretty(profile)?;
    tokio::fs::write(&path, data).await?;
    Ok(path)
}

pub async fn load_layout_profile(name: &str) -> anyhow::Result<LayoutProfile> {
    let path = layout_profile_path(name)?;
    let data = tokio::fs::read(&path)
        .await
        .with_context(|| format!("failed to read layout profile '{}'", name))?;
    serde_json::from_slice(&data)
        .with_context(|| format!("failed to parse layout profile '{}'", name))
}

pub async fn list_layout_profiles() -> anyhow::Result<Vec<LayoutProfileSummary>> {
    let dir = layout_profiles_dir();
    let mut reader = match tokio::fs::read_dir(&dir).await {
        Ok(reader) => reader,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    let mut profiles = Vec::new();
    while let Some(entry) = reader.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let Ok(data) = tokio::fs::read(&path).await else {
            continue;
        };
        let Ok(profile) = serde_json::from_slice::<LayoutProfile>(&data) else {
            continue;
        };
        profiles.push(layout_profile_summary(&profile));
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(profiles)
}

pub async fn restore_layout_profile(
    profile: &LayoutProfile,
    backend: &dyn crate::backend::DesktopBackend,
) -> anyhow::Result<serde_json::Value> {
    let current_info = backend.system_info().await?;
    let mut unmatched_windows = backend.windows_list().await?;
    let mut restored = Vec::new();
    let mut missing = Vec::new();
    let mut errors = Vec::new();

    for saved in &profile.windows {
        let Some(target_index) = match_profile_window_index(saved, &unmatched_windows) else {
            missing.push(serde_json::json!({
                "id": saved.id,
                "app_id": saved.app_id,
                "title": saved.title
            }));
            continue;
        };
        let target = unmatched_windows.remove(target_index);

        let mut window_errors = Vec::new();
        if target.workspace_id != saved.workspace_id
            && let Err(e) = backend
                .workspace_move_window(&target.id, saved.workspace_id, false)
                .await
        {
            window_errors.push(format!("workspace: {}", e));
        }
        if let Some(ref geometry) = saved.geometry
            && geometry.width > 0
            && geometry.height > 0
            && let Err(e) = backend
                .window_move_resize(
                    &target.id,
                    geometry.x,
                    geometry.y,
                    geometry.width,
                    geometry.height,
                )
                .await
        {
            window_errors.push(format!("geometry: {}", e));
        }
        if saved.is_minimized
            && let Err(e) = backend.window_minimize(&target.id).await
        {
            window_errors.push(format!("minimize: {}", e));
        }

        if window_errors.is_empty() {
            restored.push(serde_json::json!({
                "profile_window_id": saved.id,
                "window_id": target.id,
                "app_id": saved.app_id,
                "title": saved.title,
                "workspace_id": saved.workspace_id
            }));
        } else {
            errors.push(serde_json::json!({
                "profile_window_id": saved.id,
                "window_id": target.id,
                "app_id": saved.app_id,
                "title": saved.title,
                "errors": window_errors
            }));
        }
    }

    let workspace_switched = match backend.workspace_switch(profile.current_workspace).await {
        Ok(()) => true,
        Err(e) => {
            errors.push(serde_json::json!({
                "workspace_id": profile.current_workspace,
                "errors": [format!("switch: {}", e)]
            }));
            false
        }
    };

    Ok(serde_json::json!({
        "profile": profile.name,
        "restored": restored,
        "missing": missing,
        "errors": errors,
        "workspace_switched": workspace_switched,
        "current_workspace": profile.current_workspace,
        "monitor_topology_matches": monitors_match(&profile.monitors, &current_info.monitors),
        "saved_monitor_count": profile.monitors.len(),
        "current_monitor_count": current_info.monitors.len()
    }))
}

pub fn layout_profile_summary(profile: &LayoutProfile) -> LayoutProfileSummary {
    LayoutProfileSummary {
        name: profile.name.clone(),
        saved_at: profile.saved_at,
        desktop: profile.desktop.clone(),
        session_type: profile.session_type.clone(),
        current_workspace: profile.current_workspace,
        monitor_count: profile.monitors.len(),
        workspace_count: profile.workspaces.len(),
        window_count: profile.windows.len(),
    }
}

pub fn match_profile_window_index(
    saved: &crate::protocol::WindowInfo,
    current: &[crate::protocol::WindowInfo],
) -> Option<usize> {
    current
        .iter()
        .position(|w| w.id == saved.id)
        .or_else(|| {
            current.iter().position(|w| {
                !saved.app_id.is_empty()
                    && !saved.title.is_empty()
                    && w.app_id == saved.app_id
                    && w.title == saved.title
            })
        })
        .or_else(|| {
            current
                .iter()
                .position(|w| !saved.app_id.is_empty() && w.app_id == saved.app_id)
        })
        .or_else(|| {
            current
                .iter()
                .position(|w| !saved.title.is_empty() && w.title == saved.title)
        })
}

pub fn monitors_match(
    saved: &[crate::protocol::MonitorInfo],
    current: &[crate::protocol::MonitorInfo],
) -> bool {
    if saved.len() != current.len() {
        return false;
    }
    saved.iter().zip(current).all(|(a, b)| {
        a.name == b.name
            && a.width == b.width
            && a.height == b.height
            && (a.scale - b.scale).abs() < f64::EPSILON
            && a.primary == b.primary
    })
}
