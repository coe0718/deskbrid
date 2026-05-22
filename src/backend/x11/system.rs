use super::*;
use crate::protocol;
use crate::protocol::Region;

pub(super) async fn screenshot(
    backend: &X11Backend,
    _monitor: Option<u32>,
    region: Option<Region>,
    _window_id: Option<String>,
) -> anyhow::Result<protocol::ScreenshotResult> {
    let path = format!(
        "/tmp/deskbrid_x11_{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
    );
    if let Some(r) = region {
        let geo = format!("{}x{}+{}+{}", r.width, r.height, r.x, r.y);
        backend
            .sh("import", &["-window", "root", "-crop", &geo, &path])
            .await?;
        Ok(protocol::ScreenshotResult {
            path,
            width: r.width,
            height: r.height,
            format: "png".into(),
        })
    } else {
        backend.sh("import", &["-window", "root", &path]).await?;
        let dims = backend
            .sh("identify", &["-format", "%w %h", &path])
            .await
            .unwrap_or_else(|_| "0 0".into());
        let mut parts = dims.split_whitespace();
        let w: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let h: u32 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        Ok(protocol::ScreenshotResult {
            path,
            width: w,
            height: h,
            format: "png".into(),
        })
    }
}

pub(super) async fn notification_send(
    backend: &X11Backend,
    app_name: &str,
    title: &str,
    body: &str,
    urgency: &str,
) -> anyhow::Result<u32> {
    backend
        .sh("notify-send", &["-a", app_name, "-u", urgency, title, body])
        .await?;
    Ok(0)
}

pub(super) async fn notification_close(_backend: &X11Backend, _id: u32) -> anyhow::Result<()> {
    Ok(())
}

pub(super) async fn system_info(backend: &X11Backend) -> anyhow::Result<protocol::SystemInfo> {
    Ok(protocol::SystemInfo {
        desktop: backend.detected_de.clone(),
        desktop_version: "unknown".into(),
        compositor: "x11".into(),
        session_type: "x11".into(),
        monitors: backend.xrandr_monitors().await.unwrap_or_else(|_| {
            vec![protocol::MonitorInfo {
                id: 0,
                name: "X11".into(),
                width: 1920,
                height: 1080,
                scale: 1.0,
                primary: true,
                enabled: true,
                x: 0,
                y: 0,
                refresh_rate: None,
                rotation: "normal".into(),
            }]
        }),
        workspace_count: 1,
        current_workspace: 0,
        idle_seconds: 0,
    })
}

pub(super) async fn idle_seconds(_backend: &X11Backend) -> anyhow::Result<u64> {
    Ok(0)
}

pub(super) async fn power_action(_backend: &X11Backend, _action: &str) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn battery_status(
    _backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::BatteryInfo>> {
    Ok(Vec::new())
}

pub(super) async fn network_status(
    _backend: &X11Backend,
) -> anyhow::Result<protocol::NetworkStatusInfo> {
    Ok(protocol::NetworkStatusInfo {
        online: false,
        net_type: "unknown".into(),
    })
}

pub(super) async fn network_interfaces(
    _backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::NetworkInterfaceInfo>> {
    Ok(Vec::new())
}

pub(super) async fn wifi_scan(
    _backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::WifiNetworkInfo>> {
    Ok(Vec::new())
}

pub(super) async fn wifi_connect(
    _backend: &X11Backend,
    _ssid: &str,
    _password: Option<&str>,
) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn bluetooth_list(
    _backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::BluetoothDeviceInfo>> {
    Ok(Vec::new())
}

pub(super) async fn bluetooth_scan(
    _backend: &X11Backend,
    _duration: Option<u32>,
) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn bluetooth_stop_scan(_backend: &X11Backend) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn bluetooth_connect(_backend: &X11Backend, _address: &str) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn bluetooth_disconnect(
    _backend: &X11Backend,
    _address: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn files_watch(
    _backend: &X11Backend,
    _path: &str,
    _recursive: bool,
    _patterns: Option<&[String]>,
) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn files_unwatch(_backend: &X11Backend, _path: &str) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn files_search(
    _backend: &X11Backend,
    _pattern: &str,
    _root: Option<&str>,
    _max_results: u32,
) -> anyhow::Result<Vec<String>> {
    Ok(Vec::new())
}

pub(super) async fn audio_list_sinks(
    _backend: &X11Backend,
) -> anyhow::Result<Vec<protocol::AudioSinkInfo>> {
    Ok(Vec::new())
}

pub(super) async fn audio_set_sink_volume(
    _backend: &X11Backend,
    _sink_id: u32,
    _volume: f64,
) -> anyhow::Result<()> {
    anyhow::bail!("not implemented on x11 backend")
}

pub(super) async fn monitor_set_primary(backend: &X11Backend, output: &str) -> anyhow::Result<()> {
    backend
        .sh("xrandr", &["--output", output, "--primary"])
        .await
        .map(|_| ())
}

pub(super) async fn monitor_set_resolution(
    backend: &X11Backend,
    output: &str,
    width: u32,
    height: u32,
    refresh_rate: Option<f64>,
) -> anyhow::Result<()> {
    let mut args = vec![
        "--output".to_string(),
        output.to_string(),
        "--mode".into(),
        format!("{}x{}", width, height),
    ];
    if let Some(refresh) = refresh_rate {
        args.push("--rate".into());
        args.push(format_monitor_float(refresh));
    }
    backend.sh_owned("xrandr", args).await.map(|_| ())
}

pub(super) async fn monitor_set_scale(
    backend: &X11Backend,
    output: &str,
    scale: f64,
) -> anyhow::Result<()> {
    let scale_arg = format!("{0}x{0}", format_monitor_float(scale));
    backend
        .sh_owned(
            "xrandr",
            vec![
                "--output".into(),
                output.into(),
                "--scale".into(),
                scale_arg,
            ],
        )
        .await
        .map(|_| ())
}

pub(super) async fn monitor_set_rotation(
    backend: &X11Backend,
    output: &str,
    rotation: &str,
) -> anyhow::Result<()> {
    backend
        .sh(
            "xrandr",
            &["--output", output, "--rotate", xrandr_rotation(rotation)?],
        )
        .await
        .map(|_| ())
}

pub(super) async fn monitor_set_enabled(
    backend: &X11Backend,
    output: &str,
    enabled: bool,
) -> anyhow::Result<()> {
    backend
        .sh(
            "xrandr",
            &["--output", output, if enabled { "--auto" } else { "--off" }],
        )
        .await
        .map(|_| ())
}
