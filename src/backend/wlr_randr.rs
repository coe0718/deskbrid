use crate::protocol::MonitorInfo;

pub(crate) fn parse_monitors(raw: &str) -> Vec<MonitorInfo> {
    let raw = crate::util::strip_ansi(raw);
    let mut monitors = Vec::new();
    let mut current: Option<MonitorInfo> = None;

    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if !line.starts_with(char::is_whitespace) {
            if let Some(monitor) = current.take() {
                monitors.push(monitor);
            }
            let name = line.split_whitespace().next().unwrap_or("").to_string();
            current = Some(MonitorInfo {
                id: monitors.len() as u32,
                name,
                width: 0,
                height: 0,
                scale: 1.0,
                primary: false,
                enabled: false,
                x: 0,
                y: 0,
                refresh_rate: None,
                rotation: "normal".into(),
            });
            continue;
        }

        let Some(monitor) = current.as_mut() else {
            continue;
        };
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("Enabled:") {
            monitor.enabled = value.trim().eq_ignore_ascii_case("yes");
        } else if let Some(value) = trimmed.strip_prefix("Position:") {
            let mut parts = value.trim().split(',').map(str::trim);
            monitor.x = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            monitor.y = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        } else if let Some(value) = trimmed.strip_prefix("Scale:") {
            monitor.scale = value.trim().parse().unwrap_or(1.0);
        } else if let Some(value) = trimmed.strip_prefix("Transform:") {
            monitor.rotation = value.trim().to_string();
        } else if (trimmed.contains("(current") || monitor.width == 0)
            && let Some((width, height, refresh)) = parse_mode_line(trimmed)
        {
            monitor.width = width;
            monitor.height = height;
            monitor.refresh_rate = refresh;
        }
    }

    if let Some(monitor) = current.take() {
        monitors.push(monitor);
    }
    if let Some(primary) = monitors.iter_mut().find(|monitor| monitor.enabled) {
        primary.primary = true;
    }
    monitors
}

pub(crate) fn mode_arg(width: u32, height: u32, refresh_rate: Option<f64>) -> String {
    match refresh_rate {
        Some(rate) => format!("{width}x{height}@{}Hz", format_float(rate)),
        None => format!("{width}x{height}"),
    }
}

pub(crate) fn transform_arg(rotation: &str) -> anyhow::Result<&'static str> {
    match rotation {
        "normal" | "0" => Ok("normal"),
        "left" | "90" => Ok("90"),
        "right" | "270" => Ok("270"),
        "inverted" | "180" => Ok("180"),
        _ => anyhow::bail!("unsupported monitor rotation: {rotation}"),
    }
}

pub(crate) fn set_resolution_args(
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> Vec<String> {
    vec![
        "--output".into(),
        output.into(),
        "--mode".into(),
        mode_arg(width, height, refresh_rate),
    ]
}

pub(crate) fn set_scale_args(output: &str, scale: f64) -> Vec<String> {
    vec![
        "--output".into(),
        output.into(),
        "--scale".into(),
        format_float(scale),
    ]
}

pub(crate) fn set_rotation_args(output: &str, rotation: &str) -> anyhow::Result<Vec<String>> {
    Ok(vec![
        "--output".into(),
        output.into(),
        "--transform".into(),
        transform_arg(rotation)?.into(),
    ])
}

pub(crate) fn set_enabled_args(output: &str, enabled: bool) -> Vec<String> {
    vec![
        "--output".into(),
        output.into(),
        if enabled { "--on" } else { "--off" }.into(),
    ]
}

fn parse_mode_line(line: &str) -> Option<(u32, u32, Option<f64>)> {
    let dims = line.split_whitespace().next()?;
    let (width, height) = dims.split_once('x')?;
    let height = height.trim_end_matches("px");
    let refresh = line
        .split("Hz")
        .next()
        .and_then(|prefix| prefix.split_whitespace().last())
        .and_then(|value| value.parse().ok());
    Some((width.parse().ok()?, height.parse().ok()?, refresh))
}

fn format_float(value: f64) -> String {
    let mut formatted = format!("{value:.3}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_wlr_randr_outputs() {
        let raw = r#"eDP-1 "Built-in display"
  Enabled: yes
  Modes:
    1920x1080 px, 60.000 Hz (preferred, current)
    1280x720 px, 60.000 Hz
  Position: 0,0
  Transform: normal
  Scale: 1.250000
HDMI-A-1
  Enabled: no
"#;

        let monitors = super::parse_monitors(raw);
        assert_eq!(monitors.len(), 2);
        assert_eq!(monitors[0].name, "eDP-1");
        assert_eq!(monitors[0].width, 1920);
        assert_eq!(monitors[0].height, 1080);
        assert_eq!(monitors[0].refresh_rate, Some(60.0));
        assert!((monitors[0].scale - 1.25).abs() < 0.01);
        assert!(monitors[0].primary);
        assert!(!monitors[1].enabled);
    }
}
