use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;

pub(crate) async fn execute_capabilities(
    action: Action,
    backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        CapabilitiesList => {
            let actions = crate::protocol::Action::public_action_types();
            let desktop = backend.system_info().await?.desktop;
            let desktop_l = desktop.to_lowercase();
            let mut unsupported = vec![
                serde_json::json!({"action":"ui.tree.get","reason":"AT-SPI not integrated yet"}),
                serde_json::json!({"action":"ui.element.click","reason":"AT-SPI not integrated yet"}),
                serde_json::json!({"action":"ui.element.set_text","reason":"AT-SPI not integrated yet"}),
            ];
            if desktop_l.contains("hyprland") {
                unsupported.push(serde_json::json!({
                    "action":"windows.minimize",
                    "reason":"Hyprland does not expose a native minimize dispatcher"
                }));
            }
            // Keep `supported` and `unsupported` mutually exclusive for clients.
            let unsupported_actions: std::collections::HashSet<&str> = unsupported
                .iter()
                .filter_map(|entry| entry.get("action").and_then(|value| value.as_str()))
                .collect();
            let supported: Vec<&'static str> = actions
                .iter()
                .copied()
                .filter(|name| !unsupported_actions.contains(name))
                .collect();

            serde_json::json!({
                "desktop": desktop,
                "actions": actions,
                "supported": supported,
                "unsupported": unsupported
            })
        }

        _ => unreachable!("not a capabilities action"),
    })
}
