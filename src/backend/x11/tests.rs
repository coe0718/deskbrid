use super::helpers::*;

#[test]
fn parses_active_xrandr_rotation_before_capability_list() {
    assert_eq!(
        parse_xrandr_rotation(
            "DP-1 connected primary 2560x1440+0+0 right (normal left inverted right x axis y axis)"
        ),
        "right"
    );
    assert_eq!(
        parse_xrandr_rotation(
            "HDMI-1 connected 1080x1920+2560+0 inverted (normal left inverted right x axis y axis)"
        ),
        "inverted"
    );
    assert_eq!(
        parse_xrandr_rotation(
            "eDP-1 connected 1920x1080+0+0 (normal left inverted right x axis y axis)"
        ),
        "normal"
    );
}
