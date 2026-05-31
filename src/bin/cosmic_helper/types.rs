use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct WindowInfo {
    pub(crate) window_id: u64,
    pub(crate) title: Option<String>,
    pub(crate) app_id: Option<String>,
    pub(crate) pid: Option<u32>,
    pub(crate) x: Option<i32>,
    pub(crate) y: Option<i32>,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) focused: bool,
    pub(crate) minimized: bool,
    pub(crate) maximized: bool,
    pub(crate) fullscreen: bool,
    pub(crate) workspace_id: Option<u32>,
}

#[derive(Serialize, Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct WorkspaceInfo {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) is_active: bool,
}
