use super::{GnomeBackend, keysym};

impl GnomeBackend {
    pub(super) async fn keyboard_type_inner(&self, text: &str) -> anyhow::Result<()> {
        for c in text.chars() {
            let (ks, needs_shift) = keysym::from_char(c)
                .ok_or_else(|| anyhow::anyhow!("no keysym for char: {:?}", c))?;
            if needs_shift {
                self.rd_keysym(keysym::SHIFT_L, true).await?;
            }
            self.rd_keysym(ks, true).await?;
            self.rd_keysym(ks, false).await?;
            if needs_shift {
                self.rd_keysym(keysym::SHIFT_L, false).await?;
            }
        }
        Ok(())
    }

    pub(super) async fn keyboard_key_inner(&self, key: &str) -> anyhow::Result<()> {
        let ks = keysym::from_name(key)
            .or_else(|| {
                key.chars()
                    .next()
                    .and_then(|c| keysym::from_char(c).map(|(k, _)| k))
            })
            .ok_or_else(|| anyhow::anyhow!("unknown key: {}", key))?;
        self.rd_keysym(ks, true).await?;
        self.rd_keysym(ks, false).await?;
        Ok(())
    }

    pub(super) async fn keyboard_combo_inner(&self, keys: &[String]) -> anyhow::Result<()> {
        if keys.is_empty() {
            return Ok(());
        }
        let (modifiers, final_key) = keys.split_at(keys.len().saturating_sub(1));
        let final_key_str = &final_key[0];

        let mut modifier_syms: Vec<u32> = Vec::new();
        for k in modifiers {
            let sym = keysym::from_name(k)
                .or_else(|| {
                    k.chars()
                        .next()
                        .and_then(|c| keysym::from_char(c).map(|(s, _)| s))
                })
                .ok_or_else(|| anyhow::anyhow!("unknown modifier: {}", k))?;
            modifier_syms.push(sym);
        }
        let target_sym = keysym::from_name(final_key_str)
            .or_else(|| {
                final_key_str
                    .chars()
                    .next()
                    .and_then(|c| keysym::from_char(c).map(|(s, _)| s))
            })
            .ok_or_else(|| anyhow::anyhow!("unknown key: {}", final_key_str))?;

        for &sym in &modifier_syms {
            self.rd_keysym(sym, true).await?;
        }
        self.rd_keysym(target_sym, true).await?;
        self.rd_keysym(target_sym, false).await?;
        for &sym in modifier_syms.iter().rev() {
            self.rd_keysym(sym, false).await?;
        }
        Ok(())
    }

    pub(super) async fn mouse_move_inner(&self, x: f64, y: f64) -> anyhow::Result<()> {
        let (last_x, last_y) = {
            let pos = self.last_mouse.lock().unwrap();
            *pos
        };
        let dx = x - last_x;
        let dy = y - last_y;
        {
            let mut pos = self.last_mouse.lock().unwrap();
            *pos = (x, y);
        }
        self.rd_call("NotifyPointerMotionRelative", &(dx, dy))
            .await?;
        Ok(())
    }

    pub(super) async fn mouse_click_inner(&self, button: &str) -> anyhow::Result<()> {
        let btn: i32 = match button {
            "left" => 1,
            "middle" => 2,
            "right" => 3,
            _ => anyhow::bail!("unknown button: {}", button),
        };
        self.rd_button(btn, true).await?;
        self.rd_button(btn, false).await?;
        Ok(())
    }

    pub(super) async fn mouse_scroll_inner(&self, dx: f64, dy: f64) -> anyhow::Result<()> {
        if dy != 0.0 {
            self.rd_call("NotifyPointerAxisDiscrete", &(0u32, dy.round() as i32))
                .await?;
        }
        if dx != 0.0 {
            self.rd_call("NotifyPointerAxisDiscrete", &(1u32, dx.round() as i32))
                .await?;
        }
        Ok(())
    }

    pub(super) async fn mouse_drag_inner(
        &self,
        from_x: f64,
        from_y: f64,
        to_x: f64,
        to_y: f64,
        button: &str,
        duration_ms: Option<u64>,
    ) -> anyhow::Result<()> {
        let btn = mouse_button(button)?;
        self.mouse_move_inner(from_x, from_y).await?;
        self.rd_button(btn, true).await?;
        if let Some(duration_ms) = duration_ms.filter(|duration| *duration > 0) {
            tokio::time::sleep(std::time::Duration::from_millis(duration_ms.min(5_000))).await;
        }
        self.mouse_move_inner(to_x, to_y).await?;
        self.rd_button(btn, false).await?;
        Ok(())
    }
}

fn mouse_button(button: &str) -> anyhow::Result<i32> {
    match button {
        "left" => Ok(1),
        "middle" => Ok(2),
        "right" => Ok(3),
        _ => anyhow::bail!("unknown button: {}", button),
    }
}
