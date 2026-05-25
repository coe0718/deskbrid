use super::checks::{
    check_clipboard_tools, check_cmd, check_gstreamer_pipewire, check_in_path, check_process,
    check_python_gi, check_uinput,
};

pub(super) async fn insert_deps(
    desktop: &str,
    deps: &mut serde_json::Map<String, serde_json::Value>,
) {
    insert_system_deps(deps).await;

    if desktop.contains("gnome") {
        insert_gnome_deps(deps).await;
    } else if desktop.contains("kde") {
        insert_kde_deps(deps).await;
    } else if desktop.contains("hyprland") {
        insert_hyprland_deps(deps).await;
    } else if desktop.contains("cosmic") {
        insert_cosmic_deps(deps).await;
    } else if desktop.contains("sway") {
        insert_sway_deps(deps).await;
    } else if desktop.contains("niri") {
        insert_niri_deps(deps).await;
    } else if desktop.contains("wayfire") {
        insert_wayfire_deps(deps).await;
    } else if desktop.contains("labwc") {
        insert_labwc_deps(deps).await;
    } else if desktop.contains("x11") {
        insert_x11_deps(deps).await;
    }
}

async fn insert_system_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("systemctl".to_string(), check_in_path("systemctl").await);
    deps.insert("loginctl".to_string(), check_in_path("loginctl").await);
    deps.insert("journalctl".to_string(), check_in_path("journalctl").await);
    deps.insert(
        "systemd-inhibit".to_string(),
        check_in_path("systemd-inhibit").await,
    );
    deps.insert("pkcheck".to_string(), check_in_path("pkcheck").await);
    deps.insert("dm-tool".to_string(), check_in_path("dm-tool").await);
    deps.insert("tesseract".to_string(), check_in_path("tesseract").await);
}

async fn insert_gnome_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert(
        "gnome-extension".to_string(),
        check_cmd(
            "gdbus",
            &[
                "introspect",
                "--session",
                "--dest",
                "org.deskbrid.WindowManager",
                "--object-path",
                "/org/deskbrid/WindowManager",
            ],
        )
        .await,
    );
    deps.insert("grim".to_string(), check_in_path("grim").await);
    deps.insert(
        "gst_launch".to_string(),
        check_in_path("gst-launch-1.0").await,
    );
    deps.insert(
        "gstreamer_pipewire".to_string(),
        check_gstreamer_pipewire().await,
    );
    deps.insert("python_gi".to_string(), check_python_gi().await);
    deps.insert("wl_clipboard".to_string(), check_clipboard_tools().await);
    deps.insert("xrandr".to_string(), check_in_path("xrandr").await);
    deps.insert("wlr-randr".to_string(), check_in_path("wlr-randr").await);
    insert_linux_domain_deps(deps).await;
}

async fn insert_kde_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("qdbus6".to_string(), check_in_path("qdbus6").await);
    deps.insert(
        "kscreen-doctor".to_string(),
        check_in_path("kscreen-doctor").await,
    );
    deps.insert("spectacle".to_string(), check_in_path("spectacle").await);
    deps.insert(
        "imagemagick_convert".to_string(),
        check_in_path("convert").await,
    );
    deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
    deps.insert("ydotool".to_string(), check_in_path("ydotool").await);
    deps.insert("uinput".to_string(), check_uinput().await);
    insert_linux_domain_deps(deps).await;
}

async fn insert_hyprland_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("hyprctl".to_string(), check_in_path("hyprctl").await);
    deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
    deps.insert("ydotool".to_string(), check_in_path("ydotool").await);
    deps.insert("uinput".to_string(), check_uinput().await);
    deps.insert("grim".to_string(), check_in_path("grim").await);
    deps.insert("wl_clipboard".to_string(), check_clipboard_tools().await);
    deps.insert(
        "imagemagick_identify".to_string(),
        check_in_path("identify").await,
    );
    insert_linux_domain_deps(deps).await;
}

async fn insert_cosmic_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert(
        "cosmic-helper".to_string(),
        check_in_path("cosmic-helper").await,
    );
    deps.insert(
        "cosmic-randr".to_string(),
        check_in_path("cosmic-randr").await,
    );
    insert_wlroots_common_deps(deps).await;
}

async fn insert_sway_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("swaymsg".to_string(), check_in_path("swaymsg").await);
    insert_wlroots_common_deps(deps).await;
}

async fn insert_niri_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("niri".to_string(), check_in_path("niri").await);
    deps.insert("wlr-randr".to_string(), check_in_path("wlr-randr").await);
    insert_wlroots_common_deps(deps).await;
}

async fn insert_wayfire_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("wf-ipc".to_string(), check_in_path("wf-ipc").await);
    deps.insert("wlr-randr".to_string(), check_in_path("wlr-randr").await);
    insert_wlroots_common_deps(deps).await;
}

async fn insert_labwc_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("wlrctl".to_string(), check_in_path("wlrctl").await);
    deps.insert("wlr-randr".to_string(), check_in_path("wlr-randr").await);
    insert_wlroots_common_deps(deps).await;
}

async fn insert_wlroots_common_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("ydotoold".to_string(), check_process("ydotoold").await);
    deps.insert("ydotool".to_string(), check_in_path("ydotool").await);
    deps.insert("uinput".to_string(), check_uinput().await);
    deps.insert("grim".to_string(), check_in_path("grim").await);
    deps.insert("wl_clipboard".to_string(), check_clipboard_tools().await);
    deps.insert(
        "imagemagick_identify".to_string(),
        check_in_path("identify").await,
    );
    insert_linux_domain_deps(deps).await;
}

async fn insert_x11_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("xdotool".to_string(), check_in_path("xdotool").await);
    deps.insert("wmctrl".to_string(), check_in_path("wmctrl").await);
    deps.insert("xclip".to_string(), check_in_path("xclip").await);
    deps.insert("xrandr".to_string(), check_in_path("xrandr").await);
    deps.insert("import".to_string(), check_in_path("import").await);
    deps.insert("identify".to_string(), check_in_path("identify").await);
    deps.insert(
        "notify-send".to_string(),
        check_in_path("notify-send").await,
    );
    deps.insert("xprintidle".to_string(), check_in_path("xprintidle").await);
    insert_linux_domain_deps(deps).await;
}

async fn insert_linux_domain_deps(deps: &mut serde_json::Map<String, serde_json::Value>) {
    deps.insert("nmcli".to_string(), check_in_path("nmcli").await);
    deps.insert("ping".to_string(), check_in_path("ping").await);
    deps.insert(
        "bluetoothctl".to_string(),
        check_in_path("bluetoothctl").await,
    );
    deps.insert("pactl".to_string(), check_in_path("pactl").await);
    deps.insert("find".to_string(), check_in_path("find").await);
}
