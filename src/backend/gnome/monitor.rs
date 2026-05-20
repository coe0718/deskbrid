use super::GnomeBackend;

impl GnomeBackend {
    pub(super) async fn monitor_set_primary_inner(&self, output: &str) -> anyhow::Result<()> {
        if use_xrandr_session() {
            self.sh("xrandr", &["--output", output, "--primary"])
                .await?;
            return Ok(());
        }
        anyhow::bail!("setting primary monitor on GNOME Wayland requires a DisplayConfig helper")
    }

    pub(super) async fn monitor_set_resolution_inner(
        &self,
        output: &str,
        width: u32,
        height: u32,
        refresh_rate: Option<f64>,
    ) -> anyhow::Result<()> {
        let mode = format!("{}x{}", width, height);
        if use_xrandr_session() {
            let mut args = vec![
                "--output".to_string(),
                output.to_string(),
                "--mode".into(),
                mode,
            ];
            if let Some(refresh) = refresh_rate {
                args.push("--rate".into());
                args.push(format_monitor_float(refresh));
            }
            self.sh_owned("xrandr", args).await?;
            return Ok(());
        }

        let mode = if let Some(refresh) = refresh_rate {
            format!("{}x{}@{}Hz", width, height, format_monitor_float(refresh))
        } else {
            mode
        };
        self.sh_owned(
            "wlr-randr",
            vec!["--output".into(), output.into(), "--mode".into(), mode],
        )
        .await?;
        Ok(())
    }

    pub(super) async fn monitor_set_scale_inner(
        &self,
        output: &str,
        scale: f64,
    ) -> anyhow::Result<()> {
        if use_xrandr_session() {
            let scale_arg = format!("{0}x{0}", format_monitor_float(scale));
            self.sh_owned(
                "xrandr",
                vec![
                    "--output".into(),
                    output.into(),
                    "--scale".into(),
                    scale_arg,
                ],
            )
            .await?;
            return Ok(());
        }
        self.sh_owned(
            "wlr-randr",
            vec![
                "--output".into(),
                output.into(),
                "--scale".into(),
                format_monitor_float(scale),
            ],
        )
        .await?;
        Ok(())
    }

    pub(super) async fn monitor_set_rotation_inner(
        &self,
        output: &str,
        rotation: &str,
    ) -> anyhow::Result<()> {
        if use_xrandr_session() {
            self.sh(
                "xrandr",
                &["--output", output, "--rotate", xrandr_rotation(rotation)?],
            )
            .await?;
            return Ok(());
        }
        self.sh(
            "wlr-randr",
            &["--output", output, "--transform", wlr_rotation(rotation)?],
        )
        .await?;
        Ok(())
    }

    pub(super) async fn monitor_set_enabled_inner(
        &self,
        output: &str,
        enabled: bool,
    ) -> anyhow::Result<()> {
        if use_xrandr_session() {
            self.sh(
                "xrandr",
                &["--output", output, if enabled { "--auto" } else { "--off" }],
            )
            .await?;
            return Ok(());
        }
        self.sh(
            "wlr-randr",
            &["--output", output, if enabled { "--on" } else { "--off" }],
        )
        .await?;
        Ok(())
    }
}

fn xrandr_rotation(rotation: &str) -> anyhow::Result<&'static str> {
    match rotation {
        "normal" => Ok("normal"),
        "left" => Ok("left"),
        "right" => Ok("right"),
        "inverted" => Ok("inverted"),
        _ => anyhow::bail!("unsupported monitor rotation: {}", rotation),
    }
}

fn wlr_rotation(rotation: &str) -> anyhow::Result<&'static str> {
    match rotation {
        "normal" => Ok("normal"),
        "left" => Ok("90"),
        "right" => Ok("270"),
        "inverted" => Ok("180"),
        _ => anyhow::bail!("unsupported monitor rotation: {}", rotation),
    }
}

fn use_xrandr_session() -> bool {
    std::env::var("DISPLAY").is_ok() && std::env::var("WAYLAND_DISPLAY").is_err()
}

fn format_monitor_float(value: f64) -> String {
    let mut out = format!("{:.3}", value);
    while out.contains('.') && out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    out
}
