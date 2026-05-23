use super::helpers::*;
use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_process(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        // Process
        "process.list" => Action::ProcessList,
        "process.start" => Action::ProcessStart {
            command: required_string_array(raw, "command")?,
            workdir: raw["workdir"].as_str().map(String::from),
            env: raw["env"].as_object().map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            }),
        },
        "process.stop" => Action::ProcessStop {
            pid: raw["pid"].as_u64().unwrap_or(0) as u32,
            signal: raw["signal"].as_str().map(String::from),
        },
        "process.signal" => Action::ProcessSignal {
            pid: raw["pid"].as_u64().unwrap_or(0) as u32,
            signal: raw["signal"].as_str().unwrap_or("TERM").to_string(),
        },
        "process.exists" => Action::ProcessExists {
            pid: raw["pid"].as_u64().ok_or_else(|| {
                anyhow::anyhow!("missing or invalid 'pid' in process.exists request")
            })? as u32,
        },
        "process.wait" => Action::ProcessWait {
            pid: raw["pid"].as_u64().ok_or_else(|| {
                anyhow::anyhow!("missing or invalid 'pid' in process.wait request")
            })? as u32,
            timeout_ms: raw["timeout_ms"].as_u64(),
        },
        "terminal.create" => Action::TerminalCreate {
            shell: optional_non_empty_string(raw, "shell")?,
            cwd: optional_non_empty_string(raw, "cwd")?,
            env: raw["env"].as_object().map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            }),
            rows: optional_positive_u16(raw, "rows")?,
            cols: optional_positive_u16(raw, "cols")?,
        },
        "terminal.write" => Action::TerminalWrite {
            terminal_id: required_non_empty_string(raw, "terminal_id")?,
            input: raw["input"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing or invalid 'input' field"))?
                .to_string(),
        },
        "terminal.read" => Action::TerminalRead {
            terminal_id: required_non_empty_string(raw, "terminal_id")?,
            max_bytes: raw["max_bytes"].as_u64(),
            flush: raw["flush"].as_bool().unwrap_or(true),
        },
        "terminal.resize" => Action::TerminalResize {
            terminal_id: required_non_empty_string(raw, "terminal_id")?,
            rows: required_positive_u16(raw, "rows")?,
            cols: required_positive_u16(raw, "cols")?,
        },
        "terminal.list" => Action::TerminalList,
        "terminal.kill" => Action::TerminalKill {
            terminal_id: required_non_empty_string(raw, "terminal_id")?,
            signal: raw["signal"].as_str().map(String::from),
        },
        "capabilities.list" => Action::CapabilitiesList,
        _ => anyhow::bail!("unknown process type: {type_str}"),
    })
}
