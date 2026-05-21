/// Map rotation name to cosmic-randr transform value.
pub(super) fn cosmic_transform(rotation: &str) -> anyhow::Result<&'static str> {
    match rotation.to_lowercase().as_str() {
        "normal" | "none" | "0" => Ok("normal"),
        "90" | "left" => Ok("rotate90"),
        "180" | "inverted" | "flipped" => Ok("rotate180"),
        "270" | "right" => Ok("rotate270"),
        _ => anyhow::bail!("unknown rotation '{rotation}', expected: normal/90/180/270"),
    }
}

/// Format float for monitor CLI args (strip trailing zeros).
pub(super) fn format_monitor_float(value: f64) -> String {
    let mut out = format!("{:.3}", value);
    while out.contains('.') && out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}
