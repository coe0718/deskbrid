#[tokio::test]
async fn linux_session_smoke_harness() {
    if std::env::var("DESKBRID_RUN_LINUX_SESSION_TESTS")
        .ok()
        .as_deref()
        != Some("1")
    {
        eprintln!("Skipping linux session smoke tests (set DESKBRID_RUN_LINUX_SESSION_TESTS=1)");
        return;
    }

    // Harness placeholder for real DE session runners.
    // Validates test wiring without requiring GNOME/KDE/Hyprland in CI by default.
    let xdg = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    assert!(
        !xdg.is_empty(),
        "XDG_CURRENT_DESKTOP must be set when running session smoke tests"
    );
}
