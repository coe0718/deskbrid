use super::Action;

pub fn parse_desktop(raw: &serde_json::Value, _id: &str, s: &str) -> anyhow::Result<Action> {
    match s {
        "desktop.get_setting" => {
            let schema = raw["schema"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("desktop.get_setting requires schema"))?
                .to_string();
            let key = raw["key"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("desktop.get_setting requires key"))?
                .to_string();
            Ok(Action::DesktopGetSetting { schema, key })
        }
        "desktop.set_setting" => {
            let schema = raw["schema"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("desktop.set_setting requires schema"))?
                .to_string();
            let key = raw["key"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("desktop.set_setting requires key"))?
                .to_string();
            let value = raw["value"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("desktop.set_setting requires value"))?
                .to_string();
            Ok(Action::DesktopSetSetting { schema, key, value })
        }
        "desktop.list_schemas" => Ok(Action::DesktopListSchemas),
        _ => anyhow::bail!("unknown desktop action: {}", s),
    }
}
