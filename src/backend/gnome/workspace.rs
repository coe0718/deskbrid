use super::GnomeBackend;
use crate::protocol;

impl GnomeBackend {
    pub(super) async fn workspaces_list_inner(
        &self,
    ) -> anyhow::Result<Vec<protocol::WorkspaceInfo>> {
        let windows = self.windows_list_inner().await?;
        let max_ws = windows.iter().map(|w| w.workspace_id).max().unwrap_or(0) + 1;
        let current = self.get_current_workspace().await?;
        Ok((0..max_ws)
            .map(|i| protocol::WorkspaceInfo {
                id: i,
                name: format!("Workspace {}", i + 1),
                is_active: i == current,
            })
            .collect())
    }

    pub(super) async fn workspace_switch_inner(&self, id: u32) -> anyhow::Result<()> {
        self.ext_call_parsed("SwitchWorkspace", &[&id.to_string()])
            .await?;
        Ok(())
    }

    pub(super) async fn workspace_move_window_inner(
        &self,
        window_id: &str,
        workspace_id: u32,
    ) -> anyhow::Result<()> {
        let target = self.resolve_window(window_id).await?;
        self.ext_call_parsed(
            "MoveWindowToWorkspace",
            &[&target.id, &workspace_id.to_string()],
        )
        .await?;
        Ok(())
    }
}
