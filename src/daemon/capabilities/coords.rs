pub fn normalize_coords(
    info: &crate::protocol::SystemInfo,
    x: f64,
    y: f64,
    monitor: Option<u32>,
) -> serde_json::Value {
    let target = monitor
        .and_then(|m| info.monitors.iter().find(|mon| mon.id == m))
        .or_else(|| info.monitors.iter().find(|m| m.primary))
        .or_else(|| info.monitors.first());
    if let Some(mon) = target {
        let px = (x * mon.scale).round();
        let py = (y * mon.scale).round();
        serde_json::json!({
            "input": {"x": x, "y": y, "monitor": monitor},
            "monitor": {"id": mon.id, "name": mon.name, "scale": mon.scale, "width": mon.width, "height": mon.height},
            "backend_coords": {"x": px, "y": py}
        })
    } else {
        serde_json::json!({
            "input": {"x": x, "y": y, "monitor": monitor},
            "backend_coords": {"x": x, "y": y},
            "note": "no monitor metadata available"
        })
    }
}
