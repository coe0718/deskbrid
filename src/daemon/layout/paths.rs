use std::path::PathBuf;

pub fn layout_profile_path(name: &str) -> anyhow::Result<PathBuf> {
    let name = validate_layout_profile_name(name)?;
    Ok(layout_profiles_dir().join(format!("{}.json", name)))
}

pub fn layout_profiles_dir() -> PathBuf {
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home)
            .join("deskbrid")
            .join("layout_profiles");
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("deskbrid")
        .join("layout_profiles")
}

pub fn validate_layout_profile_name(name: &str) -> anyhow::Result<&str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        anyhow::bail!("layout profile name must not be empty");
    }
    if name.len() != trimmed.len() {
        anyhow::bail!("layout profile name must not start or end with whitespace");
    }
    if name == "." || name == ".." {
        anyhow::bail!("invalid layout profile name: {}", name);
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        anyhow::bail!("layout profile name may only contain letters, numbers, '.', '-' and '_'");
    }
    Ok(name)
}
