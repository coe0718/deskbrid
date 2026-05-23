use super::X11Backend;
use crate::protocol::KeyboardLayout;

impl X11Backend {
    /// Parse setxkbmap -query output:
    /// ```text
    /// rules:      evdev
    /// model:      pc105
    /// layout:     us,ru
    /// variant:    ,dvorak
    /// ```
    fn parse_setxkbmap(raw: &str) -> Vec<KeyboardLayout> {
        let mut layout_str = String::new();
        let mut variant_str = String::new();

        for line in raw.lines() {
            if let Some(val) = line.strip_prefix("layout:") {
                layout_str = val.trim().to_string();
            }
            if let Some(val) = line.strip_prefix("variant:") {
                variant_str = val.trim().to_string();
            }
        }

        let layouts: Vec<&str> = layout_str.split(',').collect();
        let variants: Vec<&str> = variant_str.split(',').collect();

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
            .collect()
    }

    pub(super) async fn keyboard_layout_list_inner(&self) -> anyhow::Result<Vec<KeyboardLayout>> {
        let output = tokio::process::Command::new("setxkbmap")
            .arg("-query")
            .output()
            .await?;
        if !output.status.success() {
            anyhow::bail!("setxkbmap -query failed");
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        Ok(Self::parse_setxkbmap(&raw))
    }

    pub(super) async fn keyboard_layout_get_inner(&self) -> anyhow::Result<KeyboardLayout> {
        let output = tokio::process::Command::new("setxkbmap")
            .arg("-query")
            .output()
            .await?;
        if !output.status.success() {
            anyhow::bail!("setxkbmap -query failed");
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        // Return first layout only
        let layouts = Self::parse_setxkbmap(&raw);
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
        let mut cmd = tokio::process::Command::new("setxkbmap");
        if let Some(n) = name {
            cmd.arg("-layout").arg(n);
        }
        if let Some(v) = variant {
            cmd.arg("-variant").arg(v);
        }
        let output = cmd.output().await?;
        if !output.status.success() {
            anyhow::bail!("setxkbmap failed");
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

        let output = tokio::process::Command::new("setxkbmap")
            .arg("-layout")
            .arg(all_names.join(","))
            .arg("-variant")
            .arg(all_variants.join(","))
            .output()
            .await?;
        if !output.status.success() {
            anyhow::bail!("setxkbmap failed");
        }
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

        let output = tokio::process::Command::new("setxkbmap")
            .arg("-layout")
            .arg(all_names.join(","))
            .arg("-variant")
            .arg(all_variants.join(","))
            .output()
            .await?;
        if !output.status.success() {
            anyhow::bail!("setxkbmap failed");
        }
        Ok(())
    }
}
