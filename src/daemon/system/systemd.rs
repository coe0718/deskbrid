use serde_json::json;

use super::command::{ensure_arg, ensure_unit, run};

pub async fn service_status(name: &str) -> anyhow::Result<serde_json::Value> {
    ensure_unit(name)?;
    let out = run(
        "systemctl",
        &[
            "show",
            name,
            "--no-pager",
            "--property=Id,Description,LoadState,ActiveState,SubState,UnitFileState",
        ],
    )
    .await?;
    let mut map = serde_json::Map::new();
    for line in out.lines() {
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.to_string(), json!(v));
        }
    }
    Ok(json!({ "name": name, "status": map }))
}

pub async fn systemctl_unit(action: &str, name: &str) -> anyhow::Result<serde_json::Value> {
    ensure_unit(name)?;
    run("systemctl", &[action, name]).await?;
    Ok(json!({ "unit": name, "action": action, "ok": true }))
}

pub async fn systemctl_enable(
    action: &str,
    name: &str,
    runtime: bool,
) -> anyhow::Result<serde_json::Value> {
    ensure_unit(name)?;
    let args = if runtime {
        vec![action, "--runtime", name]
    } else {
        vec![action, name]
    };
    run("systemctl", &args).await?;
    Ok(json!({ "unit": name, "action": action, "runtime": runtime, "ok": true }))
}

pub async fn service_list(unit_type: Option<&str>) -> anyhow::Result<serde_json::Value> {
    let unit_type = unit_type.unwrap_or("service");
    ensure_arg(unit_type, "unit_type")?;
    let kind = format!("--type={unit_type}");
    let out = run(
        "systemctl",
        &["list-units", &kind, "--all", "--no-legend", "--no-pager"],
    )
    .await?;
    let units: Vec<_> = out
        .lines()
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            json!({
                "unit": parts.first().copied().unwrap_or(""),
                "load": parts.get(1).copied().unwrap_or(""),
                "active": parts.get(2).copied().unwrap_or(""),
                "sub": parts.get(3).copied().unwrap_or(""),
                "description": if parts.len() > 4 { parts[4..].join(" ") } else { String::new() },
                "raw": line
            })
        })
        .collect();
    Ok(json!({ "unit_type": unit_type, "units": units }))
}

pub async fn journal_query(
    since: Option<u64>,
    until: Option<u64>,
    unit: Option<&str>,
    priority: Option<u8>,
    tail: Option<u32>,
) -> anyhow::Result<serde_json::Value> {
    let mut args = vec!["--no-pager".to_string(), "--output=short-iso".to_string()];
    if let Some(unit) = unit {
        ensure_unit(unit)?;
        args.push("--unit".into());
        args.push(unit.into());
    }
    if let Some(since) = since {
        args.push("--since".into());
        args.push(format!("@{since}"));
    }
    if let Some(until) = until {
        args.push("--until".into());
        args.push(format!("@{until}"));
    }
    if let Some(priority) = priority {
        if priority > 7 {
            anyhow::bail!("priority must be 0-7");
        }
        args.push("--priority".into());
        args.push(priority.to_string());
    }
    args.push("--lines".into());
    args.push(tail.unwrap_or(100).min(1000).to_string());

    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = run("journalctl", &refs).await?;
    let lines: Vec<String> = out.lines().map(ToOwned::to_owned).collect();
    Ok(json!({ "lines": lines }))
}
