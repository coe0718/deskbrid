use super::SwayBackend;
use crate::protocol::KeyboardLayout;

impl SwayBackend {
    /// Find the primary physical keyboard identifier from swaymsg get_inputs.
    /// Skips virtual devices (ydotoold), hotkey devices, and power buttons.
    fn find_keyboard_identifier(inputs: &[serde_json::Value]) -> Option<String> {
        inputs
            .iter()
            .filter(|d| {
                d.get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == "keyboard")
                    .unwrap_or(false)
            })
            .find(|d| {
                let id = d.get("identifier").and_then(|i| i.as_str()).unwrap_or("");
                // Skip virtual, hotkey, and power button devices
                !id.contains("ydotoold")
                    && !id.contains("hotkeys")
                    && !id.contains("Power_Button")
                    && !id.contains("Video_Bus")
                    && !id.contains("HP_WMI")
                    && !id.contains("Wireless")
            })
            .and_then(|d| d.get("identifier")?.as_str().map(String::from))
    }

    /// Parse xkb_layout_names from swaymsg get_inputs JSON.
    /// Returns layouts from the primary keyboard or the first keyboard with layout data.
    fn parse_sway_keyboard_layouts(
        inputs: &[serde_json::Value],
    ) -> Option<(Vec<KeyboardLayout>, u32)> {
        // Try physical keyboard first, fall back to any keyboard with layouts
        let target: &serde_json::Value = {
            let phys_id = Self::find_keyboard_identifier(inputs);
            inputs
                .iter()
                .filter(|d| {
                    d.get("type")
                        .and_then(|t| t.as_str())
                        .map(|t| t == "keyboard")
                        .unwrap_or(false)
                })
                .find(|k| {
                    phys_id.as_ref().map_or(true, |id| {
                        k.get("identifier")
                            .and_then(|i| i.as_str())
                            .map(|i| i == *id)
                            .unwrap_or(false)
                    })
                })?
        };

        let names: Vec<String> = target
            .get("xkb_layout_names")
            .and_then(|v| v.as_array())?
            .iter()
            .filter_map(|n| n.as_str().map(String::from))
            .collect();

        let active_index = target
            .get("xkb_active_layout_index")
            .and_then(|i| i.as_u64())
            .unwrap_or(0) as u32;

        if names.is_empty() {
            return None;
        }

        let layouts: Vec<KeyboardLayout> = names
            .into_iter()
            .enumerate()
            .map(|(i, name)| KeyboardLayout {
                index: i as u32,
                name,
                variant: None, // swaymsg doesn't expose active variant per-layout
                display_name: None,
            })
            .collect();

        Some((layouts, active_index))
    }

    pub(super) async fn keyboard_layout_list_inner(&self) -> anyhow::Result<Vec<KeyboardLayout>> {
        let inputs = self.swaymsg_json(&["-t", "get_inputs"]).await?;
        let inputs_arr = inputs
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("unexpected swaymsg output format"))?;
        let (layouts, _) = Self::parse_sway_keyboard_layouts(inputs_arr)
            .ok_or_else(|| anyhow::anyhow!("no keyboard layouts found in swaymsg output"))?;
        Ok(layouts)
    }

    pub(super) async fn keyboard_layout_get_inner(&self) -> anyhow::Result<KeyboardLayout> {
        let inputs = self.swaymsg_json(&["-t", "get_inputs"]).await?;
        let inputs_arr = inputs
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("unexpected swaymsg output format"))?;
        let (layouts, active_index) = Self::parse_sway_keyboard_layouts(inputs_arr)
            .ok_or_else(|| anyhow::anyhow!("no keyboard layouts found"))?;
        layouts
            .into_iter()
            .find(|l| l.index == active_index)
            .or_else(|| {
                // Fallback: return first layout if active index doesn't match
                None
            })
            .ok_or_else(|| anyhow::anyhow!("no active keyboard layout found"))
    }

    pub(super) async fn keyboard_layout_set_inner(
        &self,
        index: Option<u32>,
        name: Option<&str>,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        let inputs = self.swaymsg_json(&["-t", "get_inputs"]).await?;
        let inputs_arr = inputs
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("unexpected swaymsg output format"))?;
        let identifier = Self::find_keyboard_identifier(inputs_arr)
            .ok_or_else(|| anyhow::anyhow!("no physical keyboard found"))?;

        if let Some(idx) = index {
            // Switch to layout by index (sway IPC doesn't have a direct "switch to index N" command,
            // but we can set the layout by name)
            let (layouts, _) = Self::parse_sway_keyboard_layouts(inputs_arr)
                .ok_or_else(|| anyhow::anyhow!("could not parse keyboard layouts"))?;
            if let Some(layout) = layouts.into_iter().find(|l| l.index == idx) {
                self.swaymsg_raw(&["input", &identifier, "xkb_layout", &layout.name])
                    .await?;
            }
        }

        if let Some(n) = name {
            let layout_cmd = if let Some(v) = variant {
                self.swaymsg_raw(&["input", &identifier, "xkb_layout", n])
                    .await?;
                self.swaymsg_raw(&["input", &identifier, "xkb_variant", v])
                    .await
            } else {
                self.swaymsg_raw(&["input", &identifier, "xkb_layout", n])
                    .await
            };
            layout_cmd?;
        }

        Ok(())
    }

    pub(super) async fn keyboard_layout_add_inner(
        &self,
        name: &str,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        let inputs = self.swaymsg_json(&["-t", "get_inputs"]).await?;
        let inputs_arr = inputs
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("unexpected swaymsg output format"))?;
        let identifier = Self::find_keyboard_identifier(inputs_arr)
            .ok_or_else(|| anyhow::anyhow!("no physical keyboard found"))?;
        let (mut layouts, _) = Self::parse_sway_keyboard_layouts(inputs_arr)
            .ok_or_else(|| anyhow::anyhow!("could not parse keyboard layouts"))?;

        layouts.push(KeyboardLayout {
            index: layouts.len() as u32,
            name: name.to_string(),
            variant: variant.map(String::from),
            display_name: None,
        });

        let all_names: Vec<String> = layouts.iter().map(|l| l.name.clone()).collect();
        self.swaymsg_raw(&["input", &identifier, "xkb_layout", &all_names.join(",")])
            .await?;

        if let Some(_v) = variant {
            let all_variants: Vec<String> = layouts
                .iter()
                .map(|l| l.variant.clone().unwrap_or_default())
                .collect();
            self.swaymsg_raw(&["input", &identifier, "xkb_variant", &all_variants.join(",")])
                .await?;
        }

        Ok(())
    }

    pub(super) async fn keyboard_layout_remove_inner(&self, index: u32) -> anyhow::Result<()> {
        let inputs = self.swaymsg_json(&["-t", "get_inputs"]).await?;
        let inputs_arr = inputs
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("unexpected swaymsg output format"))?;
        let identifier = Self::find_keyboard_identifier(inputs_arr)
            .ok_or_else(|| anyhow::anyhow!("no physical keyboard found"))?;
        let (layouts, _) = Self::parse_sway_keyboard_layouts(inputs_arr)
            .ok_or_else(|| anyhow::anyhow!("could not parse keyboard layouts"))?;

        let remaining: Vec<KeyboardLayout> = layouts
            .into_iter()
            .filter(|l| l.index != index)
            .enumerate()
            .map(|(i, l)| KeyboardLayout {
                index: i as u32,
                ..l
            })
            .collect();

        let all_names: Vec<String> = remaining.iter().map(|l| l.name.clone()).collect();
        self.swaymsg_raw(&["input", &identifier, "xkb_layout", &all_names.join(",")])
            .await?;

        Ok(())
    }
}
