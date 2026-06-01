use serde_json::Value;
use std::path::{Path, PathBuf};

const CPU_ROOT: &str = "/sys/devices/system/cpu";
const THERMAL_ROOT: &str = "/sys/class/thermal";

pub async fn thermal_get() -> anyhow::Result<Value> {
    let mut entries = match tokio::fs::read_dir(THERMAL_ROOT).await {
        Ok(entries) => entries,
        Err(_) => return Ok(serde_json::json!({"zones": []})),
    };
    let mut zones = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let temp_millidegrees = read_i64(&path.join("temp")).await.unwrap_or(0);
        zones.push(serde_json::json!({
            "name": name,
            "type": read_trimmed(&path.join("type")).await.unwrap_or_default(),
            "temp_celsius": temp_millidegrees as f64 / 1000.0,
            "temp_millidegrees": temp_millidegrees,
        }));
    }
    zones.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Ok(serde_json::json!({"zones": zones}))
}

pub async fn cpu_frequency() -> anyhow::Result<Value> {
    let mut cpus = Vec::new();
    for cpu_path in cpu_paths().await? {
        let Some(cpu) = cpu_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let cpufreq = cpu_path.join("cpufreq");
        if tokio::fs::metadata(&cpufreq).await.is_err() {
            continue;
        }
        cpus.push(serde_json::json!({
            "cpu": cpu,
            "scaling_cur_freq_khz": read_u64(&cpufreq.join("scaling_cur_freq")).await.ok(),
            "cpuinfo_cur_freq_khz": read_u64(&cpufreq.join("cpuinfo_cur_freq")).await.ok(),
            "scaling_min_freq_khz": read_u64(&cpufreq.join("scaling_min_freq")).await.ok(),
            "scaling_max_freq_khz": read_u64(&cpufreq.join("scaling_max_freq")).await.ok(),
        }));
    }
    Ok(serde_json::json!({"cpus": cpus}))
}

pub async fn cpu_governor() -> anyhow::Result<Value> {
    let mut cpus = Vec::new();
    for cpu_path in cpu_paths().await? {
        let Some(cpu) = cpu_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let cpufreq = cpu_path.join("cpufreq");
        if tokio::fs::metadata(&cpufreq).await.is_err() {
            continue;
        }
        let available = read_trimmed(&cpufreq.join("scaling_available_governors"))
            .await
            .unwrap_or_default()
            .split_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();
        cpus.push(serde_json::json!({
            "cpu": cpu,
            "governor": read_trimmed(&cpufreq.join("scaling_governor")).await.ok(),
            "available_governors": available,
            "writable": writable(&cpufreq.join("scaling_governor")).await,
        }));
    }
    Ok(serde_json::json!({"cpus": cpus}))
}

pub async fn cpu_set_governor(governor: &str) -> anyhow::Result<Value> {
    let mut changed = Vec::new();
    let mut errors = Vec::new();
    for cpu_path in cpu_paths().await? {
        let Some(cpu) = cpu_path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let path = cpu_path.join("cpufreq/scaling_governor");
        if tokio::fs::metadata(&path).await.is_err() {
            continue;
        }
        match tokio::fs::write(&path, governor).await {
            Ok(_) => changed.push(cpu.to_string()),
            Err(err) => errors.push(serde_json::json!({
                "cpu": cpu,
                "error": err.to_string()
            })),
        }
    }
    if changed.is_empty() && !errors.is_empty() {
        anyhow::bail!("failed to set governor on any CPU: {}", errors[0]["error"]);
    }
    Ok(serde_json::json!({
        "governor": governor,
        "changed": changed,
        "errors": errors
    }))
}

async fn read_u64(path: &Path) -> anyhow::Result<u64> {
    Ok(tokio::fs::read_to_string(path).await?.trim().parse()?)
}

async fn read_i64(path: &Path) -> anyhow::Result<i64> {
    Ok(tokio::fs::read_to_string(path).await?.trim().parse()?)
}

async fn read_trimmed(path: &Path) -> anyhow::Result<String> {
    Ok(tokio::fs::read_to_string(path).await?.trim().to_string())
}

async fn writable(path: &Path) -> bool {
    tokio::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .await
        .is_ok()
}

async fn cpu_paths() -> anyhow::Result<Vec<PathBuf>> {
    let mut entries = match tokio::fs::read_dir(CPU_ROOT).await {
        Ok(entries) => entries,
        Err(_) => return Ok(Vec::new()),
    };
    let mut cpus = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name
            .strip_prefix("cpu")
            .is_some_and(|suffix| suffix.chars().all(|c| c.is_ascii_digit()))
        {
            cpus.push(entry.path());
        }
    }
    cpus.sort();
    Ok(cpus)
}
