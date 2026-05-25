use super::HyprBackend;
use crate::protocol::KeyboardLayout;

impl HyprBackend {
    /// Parse hyprctl devices output to extract keyboard layout config.
    /// Look for lines like `kb_layout: us,ru` inside the Keyboard section.
    fn parse_hyprctl_keyboard(raw: &str) -> Option<Vec<KeyboardLayout>> {
        let mut in_keyboard = false;
        let mut layout_str = String::new();
        let mut variant_str = String::new();

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Keyboard") {
                in_keyboard = true;
                continue;
            }
            if in_keyboard && (trimmed.starts_with("Mouse") || trimmed.starts_with("Touchpad")) {
                break;
            }
            if in_keyboard {
                if let Some(val) = trimmed.strip_prefix("kb_layout:") {
                    layout_str = val.trim().to_string();
                }
                if let Some(val) = trimmed.strip_prefix("kb_variant:") {
                    variant_str = val.trim().to_string();
                }
            }
        }

        if layout_str.is_empty() {
            return None;
        }

        let layouts: Vec<&str> = layout_str.split(',').collect();
        let variants: Vec<&str> = variant_str.split(',').collect();

        Some(
            layouts
                .into_iter()
                .enumerate()
                .map(|(i, name)| {
                    let variant = variants
                        .get(i)
                        .filter(|v| !v.is_empty())
                        .map(|v| v.to_string());
                    KeyboardLayout {
                        index: i as u32,
                        name: name.to_string(),
                        variant,
                        display_name: None,
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
