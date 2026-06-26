pub mod action;
pub mod action_impl;
pub mod action_json;
pub mod socket;
pub mod types;

#[allow(unused_imports)]
pub use action::*;
#[allow(unused_imports)]
pub use action_impl::*;
#[allow(unused_imports)]
pub use action_json::*;
#[allow(unused_imports)]
pub use socket::*;
pub use types::*;

pub mod events;
pub mod parse;
pub mod rules_types;
pub mod serialize;

pub use events::*;
pub use rules_types::*;

#[cfg(test)]
mod tests {
    use super::Action;

    #[test]
    fn parses_system_capabilities_and_health() {
        let (_, a1) = Action::from_json(r#"{"type":"system.capabilities","id":"x"}"#).unwrap();
        let (_, a2) = Action::from_json(r#"{"type":"system.health","id":"y"}"#).unwrap();
        let (_, a3) = Action::from_json(r#"{"type":"system.confinement","id":"z"}"#).unwrap();
        assert!(matches!(a1, Action::SystemCapabilities));
        assert!(matches!(a2, Action::SystemHealth));
        assert!(matches!(a3, Action::SystemConfinement));
    }

    #[test]
    fn public_actions_include_system_capabilities_and_health() {
        let actions = Action::public_action_types();
        assert!(actions.contains(&"system.capabilities"));
        assert!(actions.contains(&"system.health"));
        assert!(actions.contains(&"system.confinement"));
        assert!(actions.contains(&"windows.tile"));
        assert!(actions.contains(&"windows.activate_or_launch"));
        assert!(actions.contains(&"layout_profiles.save"));
        assert!(actions.contains(&"layout_profiles.restore"));
        assert!(actions.contains(&"monitor.set_primary"));
        assert!(actions.contains(&"monitor.set_resolution"));
        assert!(actions.contains(&"monitor.disable"));
        assert!(actions.contains(&"system.inhibit"));
        assert!(actions.contains(&"system.check_auth"));
        assert!(actions.contains(&"system.update"));
        assert!(actions.contains(&"service.restart"));
        assert!(actions.contains(&"journal.query"));
        assert!(actions.contains(&"timer.start"));
        assert!(actions.contains(&"terminal.create"));
        assert!(actions.contains(&"terminal.read"));
        assert!(actions.contains(&"wait.for"));
        assert!(actions.contains(&"screenshot.ocr"));
        assert!(actions.contains(&"screenshot.diff"));
        assert!(actions.contains(&"audit.log"));
        assert!(actions.contains(&"clipboard.history"));
        assert!(actions.contains(&"apps.list"));
        assert!(actions.contains(&"mpris.list"));
        assert!(actions.contains(&"color.pick"));
        assert!(actions.contains(&"input.mouse.drag"));
        assert!(actions.contains(&"input.layouts.list"));
        assert!(actions.contains(&"input.layout.get"));
        assert!(actions.contains(&"input.layout.set"));
        assert!(actions.contains(&"input.layout.add"));
        assert!(actions.contains(&"input.layout.remove"));
        assert!(actions.contains(&"system.backlight_get"));
        assert!(actions.contains(&"system.backlight_set"));
        assert!(actions.contains(&"system.thermal"));
        assert!(actions.contains(&"system.cpu.frequency"));
        assert!(actions.contains(&"system.cpu.governor"));
        assert!(actions.contains(&"system.cpu.set_governor"));
        assert!(actions.contains(&"network.connections.list"));
        assert!(actions.contains(&"network.connections.profiles"));
        assert!(actions.contains(&"network.hotspot.start"));
        assert!(actions.contains(&"network.hotspot.stop"));
        assert!(actions.contains(&"network.wifi.enable"));
        assert!(actions.contains(&"network.wwan.enable"));
        assert!(actions.contains(&"network.dns.set"));
        assert!(actions.contains(&"network.dns.reset"));
        assert!(actions.contains(&"network.vpn.connect"));
        assert!(actions.contains(&"network.vpn.disconnect"));
    }

    #[test]
    fn parses_mouse_drag() {
        let (_, action) = Action::from_json(
            r#"{"type":"input.mouse.drag","id":"x","from_x":1,"from_y":2,"to_x":30,"to_y":40,"button":"right","duration_ms":150}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::InputMouseDrag {
                from_x,
                from_y,
                to_x,
                to_y,
                button: Some(button),
                duration_ms: Some(150),
            } if from_x == 1.0 && from_y == 2.0 && to_x == 30.0 && to_y == 40.0 && button == "right"
        ));
        assert!(Action::from_json(r#"{"type":"input.mouse.drag","id":"x","from_x":1}"#).is_err());
    }

    #[test]
    fn parses_backlight_actions() {
        let (_, get) =
            Action::from_json(r#"{"type":"system.backlight_get","id":"x","device":"intel"}"#)
                .unwrap();
        assert!(matches!(
            get,
            Action::SystemBacklightGet {
                device: Some(device),
            } if device == "intel"
        ));

        let (_, set) =
            Action::from_json(r#"{"type":"system.backlight_set","id":"x","value":"50%"}"#).unwrap();
        assert!(matches!(
            set,
            Action::SystemBacklightSet {
                value,
                device: None,
            } if value == "50%"
        ));
        assert!(
            Action::from_json(r#"{"type":"system.backlight_set","id":"x","value":"50%"}"#).is_ok()
        );
    }

    #[test]
    fn parses_input_layout_aliases() {
        let (_, list) = Action::from_json(r#"{"type":"input.list_layouts","id":"x"}"#).unwrap();
        assert!(matches!(list, Action::InputListLayouts));

        let (_, get) = Action::from_json(r#"{"type":"input.get_layout","id":"x"}"#).unwrap();
        assert!(matches!(get, Action::InputGetLayout));

        let (_, set) =
            Action::from_json(r#"{"type":"input.set_layout","id":"x","name":"us"}"#).unwrap();
        assert!(matches!(
            set,
            Action::InputSetLayout {
                name: Some(name),
                ..
            } if name == "us"
        ));

        let (_, add) =
            Action::from_json(r#"{"type":"input.add_layout","id":"x","name":"de"}"#).unwrap();
        assert!(matches!(
            add,
            Action::InputAddLayout { name, .. } if name == "de"
        ));

        let (_, remove) =
            Action::from_json(r#"{"type":"input.remove_layout","id":"x","index":2}"#).unwrap();
        assert!(matches!(remove, Action::InputRemoveLayout { index: 2 }));
    }

    #[test]
    fn parses_thermal_and_cpu_actions() {
        let (_, thermal) = Action::from_json(r#"{"type":"system.thermal","id":"x"}"#).unwrap();
        assert!(matches!(thermal, Action::SystemThermalGet));

        let (_, freq) = Action::from_json(r#"{"type":"system.cpu.frequency","id":"x"}"#).unwrap();
        assert!(matches!(freq, Action::SystemCpuFrequency));

        let (_, governor) =
            Action::from_json(r#"{"type":"system.cpu.governor","id":"x"}"#).unwrap();
        assert!(matches!(governor, Action::SystemCpuGovernor));

        let (_, set) = Action::from_json(
            r#"{"type":"system.cpu.set_governor","id":"x","governor":"powersave"}"#,
        )
        .unwrap();
        assert!(matches!(
            set,
            Action::SystemCpuSetGovernor { governor } if governor == "powersave"
        ));
        assert!(
            Action::from_json(r#"{"type":"system.cpu.set_governor","id":"x","governor":""}"#)
                .is_err()
        );
    }

    #[test]
    fn parses_audit_actions() {
        let (_, log) = Action::from_json(
            r#"{"type":"audit.log","id":"x","limit":10,"action_type":"windows.list","status":"ok"}"#,
        )
        .unwrap();
        assert!(matches!(
            log,
            Action::AuditLog {
                limit: Some(10),
                action_type: Some(action_type),
                status: Some(status),
            } if action_type == "windows.list" && status == "ok"
        ));

        let (_, clear) = Action::from_json(r#"{"type":"audit.clear","id":"x"}"#).unwrap();
        assert!(matches!(clear, Action::AuditClear));
    }

    #[test]
    fn parses_request_options() {
        let (_, action, options) = Action::from_json_with_options(
            r#"{"type":"windows.list","id":"x","dry_run":true,"timeout_ms":250}"#,
        )
        .unwrap();
        assert!(matches!(action, Action::WindowsList));
        assert!(options.dry_run);
        assert_eq!(options.timeout_ms, Some(250));
    }

    #[test]
    fn rejects_empty_window_ids() {
        assert!(Action::from_json(r#"{"type":"windows.close","id":"x"}"#).is_err());
        assert!(Action::from_json(r#"{"type":"windows.close","id":"x","window_id":""}"#).is_err());
        assert!(
            Action::from_json(r#"{"type":"windows.move_resize","id":"x","window_id":" ","x":0,"y":0,"width":1,"height":1}"#)
                .is_err()
        );
    }

    #[test]
    fn parses_windows_tile() {
        let (_, action) = Action::from_json(
            r#"{"type":"windows.tile","id":"x","window_id":"0x1","preset":"top_left","monitor":2,"padding":8}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::WindowsTile {
                window_id,
                preset,
                monitor: Some(2),
                padding: Some(8),
            } if window_id == "0x1" && preset == "top_left"
        ));
        assert!(
            Action::from_json(r#"{"type":"windows.tile","id":"x","window_id":"0x1","preset":""}"#)
                .is_err()
        );
        assert!(
            Action::from_json(r#"{"type":"windows.tile","id":"x","window_id":"0x1","preset":"left","padding":4294967296}"#)
                .is_err()
        );
    }

    #[test]
    fn parses_windows_activate_or_launch() {
        let (_, action) = Action::from_json(
            r#"{"type":"windows.activate_or_launch","id":"x","app_id":"code","command":["code","."]}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::WindowsActivateOrLaunch {
                app_id,
                command,
                ..
            } if app_id == "code" && command == vec!["code".to_string(), ".".to_string()]
        ));
        assert!(
            Action::from_json(r#"{"type":"windows.activate_or_launch","id":"x","app_id":""}"#)
                .is_err()
        );
        assert!(
            Action::from_json(
                r#"{"type":"windows.activate_or_launch","id":"x","app_id":"code","command":[""]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn parses_layout_profile_actions() {
        let (_, save) = Action::from_json(
            r#"{"type":"layout_profiles.save","id":"x","name":"coding","overwrite":true}"#,
        )
        .unwrap();
        assert!(matches!(
            save,
            Action::LayoutProfileSave {
                name,
                overwrite: true
            } if name == "coding"
        ));

        let (_, restore) =
            Action::from_json(r#"{"type":"layout_profiles.restore","id":"x","name":"coding"}"#)
                .unwrap();
        assert!(matches!(
            restore,
            Action::LayoutProfileRestore { name } if name == "coding"
        ));
        assert!(
            Action::from_json(r#"{"type":"layout_profiles.save","id":"x","name":""}"#).is_err()
        );
    }

    #[test]
    fn parses_monitor_control_actions() {
        let (_, resolution) = Action::from_json(
            r#"{"type":"monitor.set_resolution","id":"x","output":"DP-1","width":2560,"height":1440,"refresh_rate":144}"#,
        )
        .unwrap();
        assert!(matches!(
            resolution,
            Action::MonitorSetResolution {
                output,
                width: 2560,
                height: 1440,
                refresh_rate: Some(144.0),
            } if output == "DP-1"
        ));

        let (_, rotation) = Action::from_json(
            r#"{"type":"monitor.set_rotation","id":"x","output":"eDP-1","rotation":"left"}"#,
        )
        .unwrap();
        assert!(matches!(
            rotation,
            Action::MonitorSetRotation { output, rotation }
                if output == "eDP-1" && rotation == "left"
        ));

        assert!(
            Action::from_json(r#"{"type":"monitor.set_scale","id":"x","output":"DP-1","scale":0}"#)
                .is_err()
        );
        assert!(
            Action::from_json(
                r#"{"type":"monitor.set_rotation","id":"x","output":"DP-1","rotation":"sideways"}"#
            )
            .is_err()
        );
        assert!(Action::from_json(r#"{"type":"monitor.disable","id":"x","output":""}"#).is_err());
    }

    #[test]
    fn parses_systemd_and_polkit_actions() {
        let (_, inhibit) = Action::from_json(
            r#"{"type":"system.inhibit","id":"x","what":"sleep","who":"deskbrid","why":"test","mode":"block"}"#,
        )
        .unwrap();
        assert!(matches!(
            inhibit,
            Action::SystemInhibit {
                what,
                who,
                why: Some(why),
                mode: Some(mode),
            } if what == "sleep" && who == "deskbrid" && why == "test" && mode == "block"
        ));

        let (_, service) =
            Action::from_json(r#"{"type":"service.restart","id":"x","name":"ssh.service"}"#)
                .unwrap();
        assert!(matches!(
            service,
            Action::ServiceRestart { name } if name == "ssh.service"
        ));

        let (_, journal) = Action::from_json(
            r#"{"type":"journal.query","id":"x","unit":"ssh.service","priority":3,"tail":25}"#,
        )
        .unwrap();
        assert!(matches!(
            journal,
            Action::JournalQuery {
                unit: Some(unit),
                priority: Some(3),
                tail: Some(25),
                ..
            } if unit == "ssh.service"
        ));

        let (_, elevate) = Action::from_json(
            r#"{"type":"system.elevate","id":"x","action_id":"org.deskbrid.system.service-control"}"#,
        )
        .unwrap();
        assert!(matches!(
            elevate,
            Action::SystemElevate { action_id, .. }
                if action_id == "org.deskbrid.system.service-control"
        ));

        let (_, update) =
            Action::from_json(r#"{"type":"system.update","id":"x","check":true,"force":false}"#)
                .unwrap();
        assert!(matches!(
            update,
            Action::SystemUpdate {
                check: true,
                force: false,
            }
        ));

        assert!(Action::from_json(r#"{"type":"journal.query","id":"x","priority":8}"#).is_err());
        assert!(Action::from_json(r#"{"type":"timer.start","id":"x","name":""}"#).is_err());
    }

    #[test]
    fn parses_terminal_actions() {
        let (_, create) = Action::from_json(
            r#"{"type":"terminal.create","id":"x","shell":"/bin/bash","cwd":"/tmp","rows":30,"cols":120}"#,
        )
        .unwrap();
        assert!(matches!(
            create,
            Action::TerminalCreate {
                shell: Some(shell),
                cwd: Some(cwd),
                rows: Some(30),
                cols: Some(120),
                ..
            } if shell == "/bin/bash" && cwd == "/tmp"
        ));

        let (_, read) = Action::from_json(
            r#"{"type":"terminal.read","id":"x","terminal_id":"term-1","max_bytes":4096,"flush":false}"#,
        )
        .unwrap();
        assert!(matches!(
            read,
            Action::TerminalRead {
                terminal_id,
                max_bytes: Some(4096),
                flush: false,
            } if terminal_id == "term-1"
        ));

        assert!(
            Action::from_json(
                r#"{"type":"terminal.resize","id":"x","terminal_id":"term-1","rows":0,"cols":80}"#
            )
            .is_err()
        );
        assert!(
            Action::from_json(
                r#"{"type":"terminal.write","id":"x","terminal_id":"","input":"ls\n"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn parses_wait_for_action() {
        let (_, action) = Action::from_json(
            r#"{"type":"wait.for","id":"x","condition":"file_exists","params":{"path":"/tmp/ready"},"timeout_ms":5000,"interval_ms":100}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::WaitFor {
                condition,
                timeout_ms: 5000,
                interval_ms: Some(100),
                ..
            } if condition == "file_exists"
        ));
        assert!(Action::from_json(r#"{"type":"wait.for","id":"x","condition":""}"#).is_err());
    }

    #[test]
    fn parses_screenshot_ocr_action() {
        let (_, action) = Action::from_json(
            r#"{"type":"screenshot.ocr","id":"x","path":"/tmp/s.png","language":"eng","psm":6,"bounding_boxes":true}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::ScreenshotOcr {
                path: Some(path),
                language: Some(language),
                psm: Some(6),
                bounding_boxes: true,
                ..
            } if path == "/tmp/s.png" && language == "eng"
        ));
        assert!(Action::from_json(r#"{"type":"screenshot.ocr","id":"x","path":""}"#).is_err());
    }

    #[test]
    fn parses_screenshot_diff_action() {
        let (_, action) = Action::from_json(
            r#"{"type":"screenshot.diff","id":"x","before_path":"/tmp/a.png","after_path":"/tmp/b.png","tolerance":5,"save_diff":true}"#,
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::ScreenshotDiff {
                before_path,
                after_path: Some(after_path),
                tolerance: Some(5),
                save_diff: true,
                ..
            } if before_path == "/tmp/a.png" && after_path == "/tmp/b.png"
        ));
        assert!(Action::from_json(r#"{"type":"screenshot.diff","id":"x"}"#).is_err());
    }

    #[test]
    fn serializes_events_with_event_field() {
        let event = crate::protocol::DeskbridEvent::WaitMatched {
            wait_id: "wait-1".into(),
            condition: "file_exists".into(),
            value: serde_json::json!({"path": "/tmp/ready"}),
            elapsed_ms: 25,
            timestamp: 123,
        };
        let value = serde_json::to_value(event).unwrap();
        assert_eq!(value["event"], "wait.matched");
        assert_eq!(value["wait_id"], "wait-1");
    }
}
