use super::HyprBackend;
use crate::protocol::KeyboardLayout;

impl HyprBackend {
    /// Parse hyprctl devices output to extract keyboard layout config.
    /// Handles both old format (kb_layout: us,ru) and new Hyprland 0.54+ format
    /// (rules: r "", m "", l "us", v "", o "").
    fn parse_hyprctl_keyboard(raw: &str) -> Option<Vec<KeyboardLayout>> {
        let mut layout_str = String::new();
        let mut variant_str = String::new();
        let mut keymap_str = String::new();

        for line in raw.lines() {
            let trimmed = line.trim();

            // Old format: kb_layout: us,ru / kb_variant: ,phonetic
            if let Some(val) = trimmed.strip_prefix("kb_layout:") {
                layout_str = val.trim().to_string();
            }
            if let Some(val) = trimmed.strip_prefix("kb_variant:") {
                variant_str = val.trim().to_string();
            }

            // New format (Hyprland 0.54+): rules: r "", m "", l "us", v "", o ""
            // After trim: the 'l' and '"us"' are separate tokens
            if trimmed.starts_with("rules:") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                let mut i = 0;
                while i < parts.len() {
                    match parts[i] {
                        "l" if i + 1 < parts.len() => {
                            // Next token is the quoted layout: "us" or "us,ru"
                            layout_str = parts[i + 1]
                                .trim_matches(|c: char| c == '"' || c == ',')
                                .to_string();
                            i += 1;
                        }
                        "v" if i + 1 < parts.len() => {
                            let v = parts[i + 1]
                                .trim_matches(|c: char| c == '"' || c == ',')
                                .to_string();
                            if !v.is_empty() {
                                variant_str = v;
                            }
                            i += 1;
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }

            // active keymap: English (US) — display name
            if let Some(val) = trimmed.strip_prefix("active keymap:") {
                keymap_str = val.trim().to_string();
            }
        }

        if layout_str.is_empty() {
            return None;
        }

        let layouts: Vec<&str> = layout_str.split(',').collect();
        let variants: Vec<&str> = variant_str.split(',').collect();
        let keymaps: Vec<&str> = keymap_str.split(',').collect();

        Some(
            layouts
                .into_iter()
                .enumerate()
                .map(|(i, name)| {
                    let name = name.trim();
                    let variant = variants
                        .get(i)
                        .filter(|v| !v.is_empty())
                        .map(|v| v.trim().to_string());
                    let display_name = keymaps.get(i).map(|k| k.trim().to_string());
                    KeyboardLayout {
                        index: i as u32,
                        name: name.to_string(),
                        variant,
                        display_name,
                    }
                })
                .collect(),
        )
    }

    pub(super) async fn keyboard_layout_list_inner(&self) -> anyhow::Result<Vec<KeyboardLayout>> {
        let raw = self.sh("hyprctl", &["devices"]).await?;
        Self::parse_hyprctl_keyboard(&crate::util::strip_ansi(&raw))
            .ok_or_else(|| anyhow::anyhow!("could not parse keyboard layout from hyprctl devices"))
    }

    pub(super) async fn keyboard_layout_get_inner(&self) -> anyhow::Result<KeyboardLayout> {
        let layouts = self.keyboard_layout_list_inner().await?;
        layouts
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("no keyboard layouts found"))
    }

    pub(super) async fn keyboard_layout_set_inner(
        &self,
        _index: Option<u32>,
        name: Option<&str>,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        if let Some(n) = name {
            self.hyprctl_keyword("input:kb_layout", n).await?;
        }
        if let Some(v) = variant {
            self.hyprctl_keyword("input:kb_variant", v).await?;
        }
        Ok(())
    }

    pub(super) async fn keyboard_layout_add_inner(
        &self,
        name: &str,
        variant: Option<&str>,
    ) -> anyhow::Result<()> {
        let mut layouts = self.keyboard_layout_list_inner().await?;
        layouts.push(KeyboardLayout {
            index: layouts.len() as u32,
            name: name.to_string(),
            variant: variant.map(String::from),
            display_name: None,
        });
        let all_names: Vec<String> = layouts.iter().map(|l| l.name.clone()).collect();
        let all_variants: Vec<String> = layouts
            .iter()
            .map(|l| l.variant.clone().unwrap_or_default())
            .collect();

        self.hyprctl_keyword("input:kb_layout", &all_names.join(","))
            .await?;
        self.hyprctl_keyword("input:kb_variant", &all_variants.join(","))
            .await?;
        Ok(())
    }

    pub(super) async fn keyboard_layout_remove_inner(&self, index: u32) -> anyhow::Result<()> {
        let layouts: Vec<KeyboardLayout> = self
            .keyboard_layout_list_inner()
            .await?
            .into_iter()
            .filter(|l| l.index != index)
            .collect();

        let all_names: Vec<String> = layouts.iter().map(|l| l.name.clone()).collect();
        let all_variants: Vec<String> = layouts
            .iter()
            .map(|l| l.variant.clone().unwrap_or_default())
            .collect();

        self.hyprctl_keyword("input:kb_layout", &all_names.join(","))
            .await?;
        self.hyprctl_keyword("input:kb_variant", &all_variants.join(","))
            .await?;
        Ok(())
    }
}
