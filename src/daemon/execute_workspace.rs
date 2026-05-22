use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use anyhow::Context;
use serde_json::Value;

use super::{
    capture_layout_profile, layout_profile_path, list_layout_profiles, load_layout_profile,
    restore_layout_profile, save_layout_profile,
};

pub(crate) async fn execute_workspace(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        WorkspacesList => serde_json::json!(backend.workspaces_list().await?),
        WorkspaceSwitch(id) => {
            backend.workspace_switch(id).await?;
            serde_json::json!({"workspace": id})
        }
        WorkspaceMoveWindow {
            ref window_id,
            workspace_id,
            follow,
        } => {
            backend
                .workspace_move_window(window_id, workspace_id, follow)
                .await?;
            serde_json::json!({"moved": true})
        }
        LayoutProfilesList => serde_json::json!(list_layout_profiles().await?),
        LayoutProfileGet { ref name } => serde_json::json!(load_layout_profile(name).await?),
        LayoutProfileSave {
            ref name,
            overwrite,
        } => {
            let profile = capture_layout_profile(name, backend).await?;
            let path = save_layout_profile(&profile, overwrite).await?;
            serde_json::json!({
                "profile": profile,
                "path": path.to_string_lossy()
            })
        }
        LayoutProfileDelete { ref name } => {
            let path = layout_profile_path(name)?;
            tokio::fs::remove_file(&path)
                .await
                .with_context(|| format!("failed to delete layout profile '{}'", name))?;
            serde_json::json!({"deleted": name})
        }
        LayoutProfileRestore { ref name } => {
            let profile = load_layout_profile(name).await?;
            serde_json::json!(restore_layout_profile(&profile, backend).await?)
        }

        _ => unreachable!("not a workspace action"),
    })
}
