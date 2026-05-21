use super::helpers::*;

const KSCREEN_SAMPLE: &str = r#"
Output: 1 eDP-1 enabled connected priority 1 Panel
    Geometry: 0,0 1920x1080
    Scale: 1.25
    Rotation: 1
    Modes: 0:1920x1080@60*! 1:1920x1080@48 2:1280x720@60+
Output: 2 DP-1 enabled connected priority 2 DisplayPort
    Geometry: 1920,0 2560x1440
    Scale: 1
    Modes: 0:2560x1440@144! 1:1920x1080@60+
"#;

#[test]
fn cleans_kscreen_mode_markers() {
    assert_eq!(clean_kscreen_mode_token("0:1920x1080@60*!"), "1920x1080@60");
    assert_eq!(clean_kscreen_mode_token("2:1280x720@60+"), "1280x720@60");
    assert_eq!(clean_kscreen_mode_token("2560x1440@144!"), "2560x1440@144");
}

#[test]
fn finds_kscreen_mode_without_status_markers() {
    assert_eq!(
        find_kscreen_mode(KSCREEN_SAMPLE, "eDP-1", 1920, 1080).as_deref(),
        Some("1920x1080@60")
    );
    assert_eq!(
        find_kscreen_mode(KSCREEN_SAMPLE, "DP-1", 2560, 1440).as_deref(),
        Some("2560x1440@144")
    );
}

#[test]
fn parses_kscreen_primary_from_priority_metadata() {
    let monitors = parse_kscreen_outputs(KSCREEN_SAMPLE);

    let primary = monitors.iter().find(|monitor| monitor.name == "eDP-1").unwrap();
    let secondary = monitors.iter().find(|monitor| monitor.name == "DP-1").unwrap();

    assert!(primary.primary);
    assert!(!secondary.primary);
}

#[test]
fn parses_kscreen_primary_from_colon_priority_line() {
    assert!(has_kscreen_primary_priority("Priority: 1"));
    assert!(!has_kscreen_primary_priority("Priority: 2"));
}
