pub async fn build_confinement_report() -> anyhow::Result<serde_json::Value> {
    let flatpak = detect_flatpak().await;
    let snap = detect_snap();
    let appimage = detect_appimage();
    let container = detect_container().await;
    let apparmor = detect_apparmor().await;
    let selinux = detect_selinux().await;
    let wsl = detect_wsl().await;

    let systems = vec![
        flatpak.clone(),
        snap.clone(),
        appimage.clone(),
        container.clone(),
        apparmor.clone(),
        selinux.clone(),
        wsl.clone(),
    ];
    let confined = systems
        .iter()
        .any(|system| system["confines_process"].as_bool().unwrap_or(false));

    let mut warnings = Vec::new();
    if flatpak["detected"].as_bool().unwrap_or(false) {
        warnings.push("flatpak_sandbox_detected");
    }
    if snap["detected"].as_bool().unwrap_or(false) {
        warnings.push("snap_sandbox_detected");
    }
    if container["detected"].as_bool().unwrap_or(false) {
        warnings.push("container_environment_detected");
    }
    if apparmor["confines_process"].as_bool().unwrap_or(false) {
        warnings.push("apparmor_profile_active");
    }
    if selinux["confines_process"].as_bool().unwrap_or(false) {
        warnings.push("selinux_enforcing");
    }

    Ok(serde_json::json!({
        "schema_version": 1,
        "confined": confined,
        "warnings": warnings,
        "systems": systems
    }))
}

async fn detect_flatpak() -> serde_json::Value {
    let app_id = std::env::var("FLATPAK_ID").ok();
    let info_file = tokio::fs::metadata("/.flatpak-info").await.is_ok();
    let detected = app_id.is_some() || info_file;
    serde_json::json!({
        "name": "flatpak",
        "detected": detected,
        "confines_process": detected,
        "details": {
            "app_id": app_id,
            "flatpak_info_file": info_file
        }
    })
}

fn detect_snap() -> serde_json::Value {
    let snap = std::env::var("SNAP").ok();
    let name = std::env::var("SNAP_NAME").ok();
    let detected = snap.is_some() || name.is_some();
    serde_json::json!({
        "name": "snap",
        "detected": detected,
        "confines_process": detected,
        "details": {
            "snap": snap,
            "name": name,
            "revision": std::env::var("SNAP_REVISION").ok()
        }
    })
}

fn detect_appimage() -> serde_json::Value {
    let appimage = std::env::var("APPIMAGE").ok();
    serde_json::json!({
        "name": "appimage",
        "detected": appimage.is_some(),
        "confines_process": false,
        "details": {"path": appimage}
    })
}

async fn detect_container() -> serde_json::Value {
    let env_container = std::env::var("container").ok();
    let docker_env = tokio::fs::metadata("/.dockerenv").await.is_ok();
    let container_env = tokio::fs::metadata("/run/.containerenv").await.is_ok();
    let detected = env_container.is_some() || docker_env || container_env;
    serde_json::json!({
        "name": "container",
        "detected": detected,
        "confines_process": detected,
        "details": {
            "container": env_container,
            "docker_env": docker_env,
            "container_env": container_env
        }
    })
}

async fn detect_apparmor() -> serde_json::Value {
    let enabled = tokio::fs::read_to_string("/sys/module/apparmor/parameters/enabled")
        .await
        .map(|value| value.trim().eq_ignore_ascii_case("Y"))
        .unwrap_or(false);
    let profile = tokio::fs::read_to_string("/proc/self/attr/current")
        .await
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let confined = enabled
        && profile
            .as_deref()
            .is_some_and(|value| value != "unconfined" && !value.starts_with("unconfined "));

    serde_json::json!({
        "name": "apparmor",
        "detected": enabled,
        "confines_process": confined,
        "details": {"profile": profile}
    })
}

async fn detect_selinux() -> serde_json::Value {
    let enforce = tokio::fs::read_to_string("/sys/fs/selinux/enforce")
        .await
        .ok()
        .map(|value| value.trim().to_string());
    let mode = match enforce.as_deref() {
        Some("1") => Some("enforcing"),
        Some("0") => Some("permissive"),
        Some(_) => Some("unknown"),
        None => None,
    };

    serde_json::json!({
        "name": "selinux",
        "detected": mode.is_some(),
        "confines_process": mode == Some("enforcing"),
        "details": {"mode": mode}
    })
}

async fn detect_wsl() -> serde_json::Value {
    let env_detected =
        std::env::var("WSL_INTEROP").is_ok() || std::env::var("WSL_DISTRO_NAME").is_ok();
    let release_detected = tokio::fs::read_to_string("/proc/sys/kernel/osrelease")
        .await
        .map(|value| value.to_lowercase().contains("microsoft"))
        .unwrap_or(false);
    let detected = env_detected || release_detected;

    serde_json::json!({
        "name": "wsl",
        "detected": detected,
        "confines_process": false,
        "details": {
            "distro": std::env::var("WSL_DISTRO_NAME").ok()
        }
    })
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn confinement_report_has_stable_shape() {
        let report = super::build_confinement_report().await.unwrap();
        assert_eq!(report["schema_version"], 1);
        assert!(report["confined"].is_boolean());
        assert!(report["warnings"].is_array());
        assert!(report["systems"].as_array().unwrap().len() >= 6);
    }
}
