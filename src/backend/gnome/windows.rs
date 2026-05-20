use super::GnomeBackend;
use crate::protocol;

impl GnomeBackend {
    pub(super) async fn windows_list_inner(&self) -> anyhow::Result<Vec<protocol::WindowInfo>> {
        let raw = self.ext_call_parsed("ListWindows", &[]).await?;
        super::parse_extension_json_windows(&raw)
    }

    pub(super) async fn window_focus_inner(&self, id: &str) -> anyhow::Result<()> {
        let target = self.resolve_window(id).await?;
        self.ext_call_parsed("FocusWindow", &[&target.app_id, &target.title, "true"])
            .await?;
        Ok(())
    }

    pub(super) async fn window_close_inner(&self, id: &str) -> anyhow::Result<()> {
        let target = self.resolve_window(id).await?;
        self.ext_call_bool("CloseWindow", &[&target.id]).await
    }

    pub(super) async fn window_minimize_inner(&self, id: &str) -> anyhow::Result<()> {
        let target = self.resolve_window(id).await?;
        self.ext_call_bool("MinimizeWindow", &[&target.id]).await
    }

    pub(super) async fn window_maximize_inner(&self, id: &str) -> anyhow::Result<()> {
        let target = self.resolve_window(id).await?;
        self.ext_call_bool("MaximizeWindow", &[&target.id]).await
    }

    pub(super) async fn window_move_resize_inner(
        &self,
        id: &str,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        let target = self.resolve_window(id).await?;
        self.ext_call_bool(
            "MoveResizeWindow",
            &[
                &target.id,
                &x.to_string(),
                &y.to_string(),
                &width.to_string(),
                &height.to_string(),
            ],
        )
        .await
    }
}
