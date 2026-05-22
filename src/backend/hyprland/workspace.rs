use super::*;
use crate::protocol;

pub(super) async fn workspaces_list(
    backend: &HyprBackend,
) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
    let json = backend.hyprctl_json(&["workspaces"]).await?;
    let arr = json
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("expected array"))?;
    Ok(arr
        .iter()
        .map(|w| protocol::WorkspaceInfo {
            id: w.get("id").and_then(|v| v.as_i64()).unwrap_or(0) as u32,
            name: w
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            is_active: w.get("monitor").and_then(|v| v.as_str()).is_some(),
        })
        .collect())
}

pub(super) async fn workspace_switch(backend: &HyprBackend, id: u32) -> anyhow::Result<()> {
    backend.hyprctl_dispatch(&format!("workspace {}", id)).await
}

pub(super) async fn workspace_move_window(
    backend: &HyprBackend,
    window_id: &str,
    workspace_id: u32,
    _follow: bool,
) -> anyhow::Result<()> {
    let target = backend.resolve_window(window_id).await?;
    backend
        .hyprctl_dispatch(&format!(
            "movetoworkspacesilent {},address:{}",
            workspace_id, target.id
        ))
        .await
}
