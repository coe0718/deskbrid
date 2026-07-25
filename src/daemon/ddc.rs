//! Monitor DDC/CI control — brightness, contrast, input source, power via ddcutil CLI.
//!
//! Roadmap #60. ddcutil reads/writes VCP feature codes over I2C.
//! Uses `tokio::task::spawn_blocking` so we stay async-safe.
//! Requires ddcutil ≥1.4 on the host.

use serde_json::{Value, json};

// ── Public async entry points ──────────────────

pub async fn ddc_list() -> anyhow::Result<Value> {
    tokio::task::spawn_blocking(ddc_list_blocking)
        .await
        .map_err(|e| anyhow::anyhow!("ddc_list join: {e}"))?
}

pub async fn ddc_getvcp(bus: String, vcp_code: u16) -> anyhow::Result<Value> {
    tokio::task::spawn_blocking(move || ddc_getvcp_blocking(&bus, vcp_code))
        .await
        .map_err(|e| anyhow::anyhow!("ddc_getvcp join: {e}"))?
}

pub async fn ddc_setvcp(bus: String, vcp_code: u16, value: u16) -> anyhow::Result<Value> {
    tokio::task::spawn_blocking(move || ddc_setvcp_blocking(&bus, vcp_code, value))
        .await
        .map_err(|e| anyhow::anyhow!("ddc_setvcp join: {e}"))?
}

pub async fn ddc_brightness(bus: String, percent: f64) -> anyhow::Result<Value> {
    let value = (percent.clamp(0.0, 100.0) / 100.0 * 100.0).round() as u16;
    ddc_setvcp(bus, 0x10, value).await
}

pub async fn ddc_contrast(bus: String, percent: f64) -> anyhow::Result<Value> {
    let value = (percent.clamp(0.0, 100.0) / 100.0 * 100.0).round() as u16;
    ddc_setvcp(bus, 0x12, value).await
}

pub async fn ddc_input(bus: String, input: String) -> anyhow::Result<Value> {
    let code: u16 = match input.to_ascii_lowercase().as_str() {
        "hdmi1" | "hdmi-1" => 0x11,
        "hdmi2" | "hdmi-2" => 0x12,
        "dp" | "displayport" | "dp-1" => 0x0f,
        "dp2" | "dp-2" => 0x10,
        "usb-c" | "usbc" | "typec" => 0x17,
        other => anyhow::bail!("unknown input '{other}'; expected hdmi1/hdmi2/dp/dp2/usb-c"),
    };
    ddc_setvcp(bus, 0x60, code).await
}

pub async fn ddc_power(bus: String, state: String) -> anyhow::Result<Value> {
    let code: u16 = match state.to_ascii_lowercase().as_str() {
        "on" => 0x01,
        "off" => 0x04,
        "sleep" => 0x05,
        other => anyhow::bail!("unknown power state '{other}'; try on/off/sleep"),
    };
    ddc_setvcp(bus, 0xD6, code).await
}

// ── Blocking helpers ──────────────────────────

fn ddc_list_blocking() -> anyhow::Result<Value> {
    let buses = detect_buses()?;
    let mut monitors = Vec::new();
    for bus in &buses {
        // Quick probe: can we talk to it?
        let probe = std::process::Command::new("ddcutil")
            .arg("--bus")
            .arg(bus)
            .arg("getvcp")
            .arg("10")
            .output();
        if probe.is_err() {
            continue;
        }
        let model = detect_edid_field(bus, "Model").unwrap_or_else(|_| "Unknown".into());
        let mfg = detect_edid_field(bus, "Mfg id").unwrap_or_else(|_| "Unknown".into());
        let serial = detect_edid_field(bus, "Serial number").unwrap_or_else(|_| "Unknown".into());
        let vcp_ver = detect_edid_field(bus, "VCP version").unwrap_or_else(|_| "Unknown".into());
        monitors.push(json!({
            "i2c_bus": format!("/dev/i2c-{bus}"),
            "model": model,
            "mfg_id": mfg,
            "serial": serial,
            "vcp_version": vcp_ver,
        }));
    }
    Ok(json!({"monitors": monitors, "count": monitors.len()}))
}

fn detect_buses() -> anyhow::Result<Vec<String>> {
    let det = std::process::Command::new("ddcutil")
        .arg("detect")
        .arg("--terse")
        .output()
        .map_err(|e| anyhow::anyhow!("ddcutil detect failed: {e}"))?;
    let stdout = String::from_utf8_lossy(&det.stdout);
    let mut buses = Vec::new();
    for line in stdout.lines() {
        // ddcutil --terse emits "Invalid display" prefixed lines + "I2C bus: /dev/i2c-N"
        // Only trust the second form. Otherwise, fallback form is bare integers.
        if line.contains("Invalid display") {
            continue;
        }
        for word in line.split_whitespace() {
            // pull trailing integer from forms like "/dev/i2c-7" or bare "7"
            let trailing: String = word
                .rsplit(|c: char| !c.is_ascii_digit())
                .next()
                .unwrap_or("")
                .to_string();
            if trailing.is_empty() {
                continue;
            }
            if let Ok(num) = trailing.parse::<u8>() {
                if num >= 1 && num <= 32 {
                    let s = num.to_string();
                    if !buses.contains(&s) {
                        buses.push(s);
                    }
                }
            }
        }
    }
    if buses.is_empty() {
        anyhow::bail!("no DDC/CI monitors found (ddcutil detect returned nothing)");
    }
    Ok(buses)
}

fn detect_edid_field(bus: &str, field: &str) -> anyhow::Result<String> {
    let out = std::process::Command::new("ddcutil")
        .arg("detect")
        .arg("--bus")
        .arg(bus)
        .output()
        .map_err(|e| anyhow::anyhow!("ddcutil detect bus {bus}: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if line.contains(field) {
            return Ok(line.rsplit(':').next().unwrap_or("").trim().to_string());
        }
    }
    Ok("Unknown".into())
}

fn ddc_getvcp_blocking(bus: &str, vcp_code: u16) -> anyhow::Result<Value> {
    let vcp_arg = format!("{vcp_code}");
    let cmd = std::process::Command::new("ddcutil")
        .arg("getvcp")
        .arg(&vcp_arg)
        .arg("--bus")
        .arg(bus)
        .output()
        .map_err(|e| anyhow::anyhow!("ddcutil {bus} getvcp {vcp_arg}: {e}"))?;
    let stdout = String::from_utf8_lossy(&cmd.stdout);
    let stderr = String::from_utf8_lossy(&cmd.stderr);

    if !cmd.status.success() && !stderr.is_empty() && !stderr.contains("DDC communication") {
        anyhow::bail!("ddcutil getvcp {vcp_arg}: {stderr}");
    }
    // Parse "VCP 10 80 100" → current=80, max=100
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] == "VCP" {
            let current: u16 = parts[2].parse().unwrap_or(0);
            let max: Option<u16> = parts.get(3).and_then(|p| p.parse().ok());
            return Ok(json!({"bus": bus, "vcp_code": vcp_code, "value": current, "max": max}));
        }
    }
    Ok(json!({"bus": bus, "vcp_code": vcp_code, "value": null, "raw": stdout.trim()}))
}

fn ddc_setvcp_blocking(bus: &str, vcp_code: u16, value: u16) -> anyhow::Result<Value> {
    let vcp_arg = format!("{vcp_code}");
    let val_arg = format!("{value}");
    let cmd = std::process::Command::new("ddcutil")
        .arg("setvcp")
        .arg(&vcp_arg)
        .arg(&val_arg)
        .arg("--bus")
        .arg(bus)
        .output()
        .map_err(|e| anyhow::anyhow!("ddcutil {bus} setvcp {vcp_arg} {val_arg}: {e}"))?;
    let stderr = String::from_utf8_lossy(&cmd.stderr);
    if !cmd.status.success() {
        anyhow::bail!("ddcutil setvcp {vcp_arg} {val_arg}: {stderr}");
    }
    Ok(json!({"bus": bus, "vcp_code": vcp_code, "value": value, "success": true}))
}
