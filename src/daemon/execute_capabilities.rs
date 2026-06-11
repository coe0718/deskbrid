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

            // High-risk actions that require explicit allow-listing (never authorized by wildcards)
            let high_risk: Vec<&str> = crate::permissions::HIGH_RISK_ACTIONS.to_vec();

            // Sandbox: file access is restricted to these colon-separated directories
            let sandbox_dirs: Vec<String> = std::env::var("DESKBRID_ALLOWED_DIRS")
                .unwrap_or_else(|_| {
                    let home = dirs::home_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "/root".to_string());
                    format!("{}:/tmp", home)
                })
                .split(':')
                .map(|s| s.to_string())
                .collect();

            // Permissions file location
            let permissions_path = dirs::config_dir()
                .map(|p| p.join("deskbrid").join("permissions.toml"))
                .map(|p| p.to_string_lossy().to_string());

            serde_json::json!({
                "desktop": desktop,
                "actions": actions,
                "supported": supported,
                "unsupported": unsupported,
                "high_risk": high_risk,
                "sandbox": {
                    "mechanism": "DESKBRID_ALLOWED_DIRS",
                    "dirs": sandbox_dirs,
                },
                "transport": {
                    "type": "unix_socket",
                    "local_only": true,
                    "socket_path": format!("$XDG_RUNTIME_DIR/deskbrid.sock"),
                },
                "permissions": {
                    "path": permissions_path,
                    "model": "deny_wins_first_then_allow",
                    "high_risk_policy": "never_auth_by_wildcard_must_be_explicitly_named",
                }
            })
        }

        _ => anyhow::bail!("internal dispatch error: not a capabilities action"),
    })
}
