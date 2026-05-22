//! uinput Absolute Pointer — zero-latency pixel-precise mouse control.

use anyhow::Context;
use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, InputEvent, KeyCode, SynchronizationCode,
    UinputAbsSetup, uinput::VirtualDevice,
};

pub struct AbsPointer {
    device: VirtualDevice,
    screen_width: u32,
    screen_height: u32,
}

impl AbsPointer {
    pub fn new(screen_width: u32, screen_height: u32) -> anyhow::Result<Self> {
        let mut builder = VirtualDevice::builder()
            .context("failed to open /dev/uinput — is the uinput kernel module loaded?")?;

        builder = builder.name("deskbrid-uinput-pointer");

        let abs_x = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_X,
            AbsInfo::new(0, 0, screen_width as i32, 0, 0, 0),
        );
        builder = builder
            .with_absolute_axis(&abs_x)
            .context("failed to set ABS_X")?;

        let abs_y = UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_Y,
            AbsInfo::new(0, 0, screen_height as i32, 0, 0, 0),
        );
        builder = builder
            .with_absolute_axis(&abs_y)
            .context("failed to set ABS_Y")?;

        let keys: AttributeSet<KeyCode> =
            [KeyCode::BTN_LEFT, KeyCode::BTN_RIGHT, KeyCode::BTN_MIDDLE]
                .into_iter()
                .collect();
        builder = builder.with_keys(&keys).context("failed to set key bits")?;

        let device = builder.build().context("failed to create uinput device")?;

        Ok(Self {
            device,
            screen_width,
            screen_height,
        })
    }

    pub fn move_to(&mut self, x: f64, y: f64) -> anyhow::Result<()> {
        let ax = x.clamp(0.0, self.screen_width as f64) as i32;
        let ay = y.clamp(0.0, self.screen_height as f64) as i32;
        self.device.emit(&[
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, ax),
            InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, ay),
            InputEvent::new(
                EventType::SYNCHRONIZATION.0,
                SynchronizationCode::SYN_REPORT.0,
                0,
            ),
        ])?;
        Ok(())
    }

    pub fn click(&mut self, button_code: u16) -> anyhow::Result<()> {
        self.device.emit(&[
            InputEvent::new(EventType::KEY.0, button_code, 1),
            InputEvent::new(
                EventType::SYNCHRONIZATION.0,
                SynchronizationCode::SYN_REPORT.0,
                0,
            ),
        ])?;
        self.device.emit(&[
            InputEvent::new(EventType::KEY.0, button_code, 0),
            InputEvent::new(
                EventType::SYNCHRONIZATION.0,
                SynchronizationCode::SYN_REPORT.0,
                0,
            ),
        ])?;
        Ok(())
    }

    pub fn click_at(&mut self, x: f64, y: f64, button_code: u16) -> anyhow::Result<()> {
        self.move_to(x, y)?;
        std::thread::sleep(std::time::Duration::from_millis(5));
        self.click(button_code)?;
        Ok(())
    }

    pub fn drag(
        &mut self,
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        button_code: u16,
    ) -> anyhow::Result<()> {
        self.move_to(from_x, from_y)?;
        std::thread::sleep(std::time::Duration::from_millis(5));
        self.device.emit(&[
            InputEvent::new(EventType::KEY.0, button_code, 1),
            InputEvent::new(
                EventType::SYNCHRONIZATION.0,
                SynchronizationCode::SYN_REPORT.0,
                0,
            ),
        ])?;
        std::thread::sleep(std::time::Duration::from_millis(2));

        let steps = 20;
        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let cx = from_x + (to_x - from_x) * t;
            let cy = from_y + (to_y - from_y) * t;
            let ax = cx.clamp(0.0, self.screen_width as f64) as i32;
            let ay = cy.clamp(0.0, self.screen_height as f64) as i32;
            self.device.emit(&[
                InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_X.0, ax),
                InputEvent::new(EventType::ABSOLUTE.0, AbsoluteAxisCode::ABS_Y.0, ay),
                InputEvent::new(
                    EventType::SYNCHRONIZATION.0,
                    SynchronizationCode::SYN_REPORT.0,
                    0,
                ),
            ])?;
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        self.device.emit(&[
            InputEvent::new(EventType::KEY.0, button_code, 0),
            InputEvent::new(
                EventType::SYNCHRONIZATION.0,
                SynchronizationCode::SYN_REPORT.0,
                0,
            ),
        ])?;
        Ok(())
    }
}

pub fn create_for_screen() -> Option<AbsPointer> {
    let (w, h) = detect_screen_dimensions();
    AbsPointer::new(w, h).ok()
}

fn detect_screen_dimensions() -> (u32, u32) {
    if let Ok(output) = std::process::Command::new("xrandr")
        .args(["--current"])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            if line.contains(" connected")
                && let Some(res) = line.find(|c: char| c.is_ascii_digit())
            {
                let rest = &line[res..];
                if let Some(space) = rest.find(' ') {
                    let res_str = &rest[..space];
                    let parts: Vec<&str> = res_str.split('x').collect();
                    if parts.len() == 2
                        && let (Ok(w), Ok(h)) = (parts[0].parse(), parts[1].parse())
                    {
                        return (w, h);
                    }
                }
            }
        }
    }
    (1920, 1080)
}

pub fn button_code(name: &str) -> u16 {
    match name.to_lowercase().as_str() {
        "left" => KeyCode::BTN_LEFT.0,
        "right" => KeyCode::BTN_RIGHT.0,
        "middle" => KeyCode::BTN_MIDDLE.0,
        _ => KeyCode::BTN_LEFT.0,
    }
}
