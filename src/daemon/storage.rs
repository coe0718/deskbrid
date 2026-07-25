//! Storage monitoring — filesystem usage and large-entry scans.
//!
//! Roadmap #95. Pure Linux (`statvfs` + `/proc/mounts` + directory walk).
//! No DE backends, no new crates.

use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

/// Filesystem types that are virtual / not useful for capacity decisions.
const SKIP_FS_TYPES: &[&str] = &[
    "autofs",
    "binfmt_misc",
    "bpf",
    "cgroup",
    "cgroup2",
    "configfs",
    "debugfs",
    "devpts",
    "devtmpfs",
    "efivarfs",
    "fusectl",
    "hugetlbfs",
    "mqueue",
    "nsfs",
    "proc",
    "pstore",
    "rpc_pipefs",
    "securityfs",
    "sysfs",
    "tracefs",
];

#[derive(Debug, Clone, Serialize)]
pub struct StorageUsageInfo {
    pub path: String,
    pub mount_point: String,
    pub filesystem: String,
    pub device: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
    pub percent_used: f64,
    pub warning: bool,
    pub critical: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StorageScanEntry {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub size_bytes: u64,
}

/// `storage.usage` — one path or every real mount.
pub async fn storage_usage(path: Option<String>) -> anyhow::Result<Value> {
    tokio::task::spawn_blocking(move || storage_usage_blocking(path))
        .await
        .map_err(|e| anyhow::anyhow!("storage.usage join error: {e}"))?
}

/// `storage.scan` — largest entries under a path.
pub async fn storage_scan(
    path: String,
    max_depth: Option<u32>,
    limit: Option<u32>,
) -> anyhow::Result<Value> {
    tokio::task::spawn_blocking(move || storage_scan_blocking(path, max_depth, limit))
        .await
        .map_err(|e| anyhow::anyhow!("storage.scan join error: {e}"))?
}

fn storage_usage_blocking(path: Option<String>) -> anyhow::Result<Value> {
    if let Some(raw) = path {
        let p = expand_user_path(&raw)?;
        if !p.exists() {
            anyhow::bail!("path does not exist: {}", p.display());
        }
        let info = usage_for_path(&p)?;
        return Ok(json!({ "mounts": [info], "count": 1 }));
    }

    let mounts = list_real_mounts()?;
    let mut out = Vec::with_capacity(mounts.len());
    let mut seen_devices = HashSet::new();
    for m in mounts {
        // Dedup by device+mount so bind mounts of the same volume don't spam.
        let key = format!("{}:{}", m.device, m.mount_point);
        if !seen_devices.insert(key) {
            continue;
        }
        match usage_for_mount(&m) {
            Ok(info) => out.push(info),
            Err(_) => continue,
        }
    }
    // Critical / fullest first — agents care about risk.
    out.sort_by(|a, b| {
        b.percent_used
            .partial_cmp(&a.percent_used)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.mount_point.cmp(&b.mount_point))
    });
    let count = out.len();
    let warnings = out.iter().filter(|m| m.warning).count();
    let criticals = out.iter().filter(|m| m.critical).count();
    Ok(json!({
        "mounts": out,
        "count": count,
        "warnings": warnings,
        "criticals": criticals,
    }))
}

fn storage_scan_blocking(
    path: String,
    max_depth: Option<u32>,
    limit: Option<u32>,
) -> anyhow::Result<Value> {
    let root = expand_user_path(&path)?;
    if !root.exists() {
        anyhow::bail!("path does not exist: {}", root.display());
    }
    if !root.is_dir() {
        anyhow::bail!("storage.scan requires a directory path");
    }

    let max_depth = max_depth.unwrap_or(1).min(8);
    let limit = limit.unwrap_or(25).clamp(1, 500) as usize;

    let mut entries = Vec::new();
    let read = fs::read_dir(&root)
        .map_err(|e| anyhow::anyhow!("failed to read directory {}: {e}", root.display()))?;
    for ent in read.flatten() {
        let p = ent.path();
        let name = ent.file_name().to_string_lossy().into_owned();
        let meta = match ent.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let kind = if meta.is_dir() {
            "directory"
        } else if meta.is_file() {
            "file"
        } else if meta.file_type().is_symlink() {
            "symlink"
        } else {
            "other"
        };
        let size = if meta.is_dir() {
            dir_size(&p, 0, max_depth, &mut 0)
        } else {
            meta.len()
        };
        entries.push(StorageScanEntry {
            path: p.display().to_string(),
            name,
            kind: kind.into(),
            size_bytes: size,
        });
    }
    entries.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.path.cmp(&b.path))
    });
    entries.truncate(limit);

    let total_listed: u64 = entries.iter().map(|e| e.size_bytes).sum();
    Ok(json!({
        "path": root.display().to_string(),
        "max_depth": max_depth,
        "limit": limit,
        "entries": entries,
        "total_listed_bytes": total_listed,
        "count": entries.len(),
    }))
}

#[derive(Debug, Clone)]
struct MountEntry {
    device: String,
    mount_point: String,
    filesystem: String,
}

fn list_real_mounts() -> anyhow::Result<Vec<MountEntry>> {
    let content = fs::read_to_string("/proc/mounts")
        .or_else(|_| fs::read_to_string("/proc/self/mounts"))
        .map_err(|e| anyhow::anyhow!("failed to read /proc/mounts: {e}"))?;
    let mut out = Vec::new();
    for line in content.lines() {
        let mut parts = line.split_whitespace();
        let Some(device) = parts.next() else { continue };
        let Some(mount_point) = parts.next() else {
            continue;
        };
        let Some(fs_type) = parts.next() else {
            continue;
        };
        if SKIP_FS_TYPES.contains(&fs_type) {
            continue;
        }
        // Skip kernel/virtual mount points even if fs type is unusual.
        if mount_point.starts_with("/proc")
            || mount_point.starts_with("/sys")
            || mount_point == "/dev"
            || mount_point.starts_with("/dev/")
            || mount_point.starts_with("/run/user/")
        {
            continue;
        }
        out.push(MountEntry {
            device: unescape_mount(device),
            mount_point: unescape_mount(mount_point),
            filesystem: fs_type.to_string(),
        });
    }
    Ok(out)
}

fn unescape_mount(s: &str) -> String {
    // /proc/mounts escapes space as \040, tab \011, newline \012, backslash \134.
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut oct = String::new();
            for _ in 0..3 {
                if let Some(d) = chars.peek().copied()
                    && d.is_ascii_digit()
                {
                    oct.push(chars.next().unwrap());
                    continue;
                }
                break;
            }
            if oct.len() == 3
                && let Ok(v) = u8::from_str_radix(&oct, 8)
            {
                out.push(v as char);
                continue;
            }
            out.push('\\');
            out.push_str(&oct);
        } else {
            out.push(c);
        }
    }
    out
}

fn usage_for_path(path: &Path) -> anyhow::Result<StorageUsageInfo> {
    let st = statvfs(path)?;
    let mount_point = find_mount_point(path).unwrap_or_else(|| path.display().to_string());
    let (device, filesystem) =
        mount_meta_for(&mount_point).unwrap_or_else(|| ("unknown".into(), "unknown".into()));
    Ok(build_usage(
        path.display().to_string(),
        mount_point,
        filesystem,
        device,
        st,
    ))
}

fn usage_for_mount(m: &MountEntry) -> anyhow::Result<StorageUsageInfo> {
    let path = PathBuf::from(&m.mount_point);
    let st = statvfs(&path)?;
    Ok(build_usage(
        m.mount_point.clone(),
        m.mount_point.clone(),
        m.filesystem.clone(),
        m.device.clone(),
        st,
    ))
}

struct StatVfs {
    total: u64,
    free: u64,
    available: u64,
}

fn statvfs(path: &Path) -> anyhow::Result<StatVfs> {
    let c_path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| anyhow::anyhow!("path contains interior NUL: {}", path.display()))?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut st) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        anyhow::bail!("statvfs({}) failed: {err}", path.display());
    }
    let frsize = st.f_frsize as u64;
    let total = st.f_blocks.saturating_mul(frsize);
    let free = st.f_bfree.saturating_mul(frsize);
    let available = st.f_bavail.saturating_mul(frsize);
    Ok(StatVfs {
        total,
        free,
        available,
    })
}

fn build_usage(
    path: String,
    mount_point: String,
    filesystem: String,
    device: String,
    st: StatVfs,
) -> StorageUsageInfo {
    let used = st.total.saturating_sub(st.free);
    let percent = if st.total == 0 {
        0.0
    } else {
        (used as f64 / st.total as f64) * 100.0
    };
    StorageUsageInfo {
        path,
        mount_point,
        filesystem,
        device,
        total_bytes: st.total,
        used_bytes: used,
        free_bytes: st.free,
        available_bytes: st.available,
        percent_used: (percent * 100.0).round() / 100.0,
        warning: percent >= 90.0,
        critical: percent >= 95.0,
    }
}

fn mount_meta_for(mount_point: &str) -> Option<(String, String)> {
    let mounts = list_real_mounts().ok()?;
    mounts
        .into_iter()
        .find(|m| m.mount_point == mount_point)
        .map(|m| (m.device, m.filesystem))
}

fn find_mount_point(path: &Path) -> Option<String> {
    let mounts = list_real_mounts().ok()?;
    let canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut best: Option<String> = None;
    let mut best_len = 0usize;
    for m in mounts {
        let mp = PathBuf::from(&m.mount_point);
        if canon.starts_with(&mp) && m.mount_point.len() >= best_len {
            best_len = m.mount_point.len();
            best = Some(m.mount_point);
        }
    }
    best
}

/// Recursive directory size with depth + visit caps so agents can't hang the daemon.
fn dir_size(path: &Path, depth: u32, max_depth: u32, visits: &mut u32) -> u64 {
    const MAX_VISITS: u32 = 50_000;
    if *visits >= MAX_VISITS {
        return 0;
    }
    *visits += 1;
    if depth > max_depth {
        // Past the requested depth, return metadata size only (dir entry itself).
        return fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }
    let read = match fs::read_dir(path) {
        Ok(r) => r,
        Err(_) => return 0,
    };
    let mut total = 0u64;
    for ent in read.flatten() {
        if *visits >= MAX_VISITS {
            break;
        }
        let p = ent.path();
        let meta = match ent.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.is_dir() {
            // Don't follow symlinked directories — prevents loops / escape.
            if meta.file_type().is_symlink() {
                continue;
            }
            total = total.saturating_add(dir_size(&p, depth + 1, max_depth, visits));
        } else {
            *visits += 1;
            total = total.saturating_add(meta.len());
        }
    }
    total
}

fn expand_user_path(raw: &str) -> anyhow::Result<PathBuf> {
    if let Some(rest) = raw.strip_prefix("~/") {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        return Ok(home.join(rest));
    }
    if raw == "~" {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        return Ok(home);
    }
    Ok(PathBuf::from(raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn unescape_space_in_mount() {
        assert_eq!(unescape_mount("/mnt/my\\040disk"), "/mnt/my disk");
    }

    #[test]
    fn usage_root_works() {
        let info = usage_for_path(Path::new("/")).expect("root statvfs");
        assert!(info.total_bytes > 0);
        assert!(info.percent_used >= 0.0 && info.percent_used <= 100.0);
        assert!(!info.mount_point.is_empty());
    }

    #[test]
    fn scan_tmp_dir() {
        let dir = tempfile_dir();
        let mut f = fs::File::create(dir.join("big.bin")).unwrap();
        f.write_all(&vec![0u8; 4096]).unwrap();
        drop(f);
        fs::create_dir(dir.join("subdir")).unwrap();
        fs::write(dir.join("subdir/a.txt"), b"hello").unwrap();

        let result = storage_scan_blocking(dir.display().to_string(), Some(2), Some(10)).unwrap();
        assert_eq!(result["path"], dir.display().to_string());
        let entries = result["entries"].as_array().unwrap();
        assert!(!entries.is_empty());
        // big.bin should be among the largest
        let names: Vec<&str> = entries.iter().filter_map(|e| e["name"].as_str()).collect();
        assert!(names.contains(&"big.bin"));
        let _ = fs::remove_dir_all(&dir);
    }

    fn tempfile_dir() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "deskbrid-storage-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
}
