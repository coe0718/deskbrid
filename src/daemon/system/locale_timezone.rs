//! Locale and timezone read/write via standard Linux config files.
//!
//! This module reads from `std::env` (process) and `/etc/locale.conf`
//! (persistent), and writes to `/etc/locale.conf` and the
//! `/etc/localtime` symlink. The persistent writes require root on
//! most systems — the function returns a clean error rather than
//! silently failing.
//!
//! We do NOT use `localectl` / `timedatectl` because they require
//! systemd, may require polkit, and are not universal. The file-level
//! approach works on systemd, sysvinit, OpenRC, and Alpine.

use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

/// Canonical list of locale variables we read & report on.
const LOCALE_KEYS: &[&str] = &[
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LC_CTYPE",
    "LC_TIME",
    "LC_NUMERIC",
    "LC_MONETARY",
    "LC_COLLATE",
    "LC_PAPER",
    "LC_NAME",
    "LC_ADDRESS",
    "LC_TELEPHONE",
    "LC_MEASUREMENT",
    "LC_IDENTIFICATION",
    "LC_MESSAGES",
];

const LOCALE_CONF: &str = "/etc/locale.conf";
const ETC_TIMEZONE: &str = "/etc/timezone";
const ETC_LOCALTIME: &str = "/etc/localtime";
const ZONEINFO_DIR: &str = "/usr/share/zoneinfo";

// ---------- Locale ----------

/// Read the current locale state from process env (primary) and fall
/// back to /etc/locale.conf for anything missing. Reports both sources
/// so callers can distinguish "what my process sees" from "what the
/// persistent config says".
pub(super) fn locale_get() -> Value {
    let persistent = read_locale_conf().unwrap_or_default();
    let mut values = serde_json::Map::with_capacity(LOCALE_KEYS.len());
    let mut sources = serde_json::Map::with_capacity(LOCALE_KEYS.len());
    for &k in LOCALE_KEYS {
        let from_proc = std::env::var(k).ok();
        let from_file = persistent.get(k).cloned();
        match (from_proc, from_file) {
            (Some(p), _) => {
                values.insert(k.to_string(), Value::String(p));
                sources.insert(k.to_string(), Value::String("process".into()));
            }
            (None, Some(f)) => {
                values.insert(k.to_string(), Value::String(f));
                sources.insert(k.to_string(), Value::String(LOCALE_CONF.into()));
            }
            (None, None) => {
                values.insert(k.to_string(), Value::Null);
                sources.insert(k.to_string(), Value::String("unset".into()));
            }
        }
    }
    let set_count = values.values().filter(|v| !v.is_null()).count();
    json!({
        "values": Value::Object(values),
        "sources": Value::Object(sources),
        "available": LOCALE_KEYS,
        "set_count": set_count,
        "persistent_file": LOCALE_CONF,
    })
}

/// Write locale vars to /etc/locale.conf. Caller must pass an
/// allow-list — keys outside `LOCALE_KEYS` are rejected.
pub(super) fn locale_set(vars: &[(String, String)]) -> Value {
    if vars.is_empty() {
        return json!({
            "written": {},
            "requires_root": true,
            "error": "no vars to set",
        });
    }
    // Validate every key against the allow-list
    for (k, _) in vars {
        if !LOCALE_KEYS.contains(&k.as_str()) {
            return json!({
                "written": {},
                "requires_root": true,
                "error": format!("unknown locale key {:?}; expected one of {:?}", k, LOCALE_KEYS),
            });
        }
    }
    // Read existing persistent file (if any) to preserve unknown lines
    let existing_lines: Vec<String> = match fs::read_to_string(LOCALE_CONF) {
        Ok(s) => s.lines().map(String::from).collect(),
        Err(_) => Vec::new(),
    };
    let mut out_lines: Vec<String> = Vec::with_capacity(existing_lines.len());
    let mut replaced: Vec<String> = Vec::new();
    let mut preserved_unknown: Vec<String> = Vec::new();
    let mut requested: Vec<(String, String)> = vars.to_vec();

    for line in &existing_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            out_lines.push(line.clone());
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let k = &trimmed[..eq];
            // Strip optional leading "export "
            let k = k.trim_start_matches("export ").trim();
            if let Some(idx) = requested.iter().position(|(rk, _)| rk == k) {
                let (_, v) = &requested[idx];
                let value = quote_value(v);
                out_lines.push(format!("{}=\"{}\"", k, value));
                replaced.push(k.to_string());
                requested.remove(idx);
            } else if LOCALE_KEYS.contains(&k) {
                // Existing locale key but not in this request — keep its current value
                out_lines.push(line.clone());
            } else {
                // Unknown key (not in our allow-list) — preserve as-is
                out_lines.push(line.clone());
                preserved_unknown.push(k.to_string());
            }
        } else {
            out_lines.push(line.clone());
        }
    }

    // Append any keys that weren't already in the file
    for (k, v) in requested {
        let value = quote_value(&v);
        out_lines.push(format!("{}=\"{}\"", k, value));
        replaced.push(k);
    }

    let content = out_lines.join("\n") + "\n";
    match atomic_write(Path::new(LOCALE_CONF), &content) {
        Ok(()) => {
            let written: serde_json::Map<String, Value> = vars
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            json!({
                "written": Value::Object(written),
                "source": LOCALE_CONF,
                "requires_root": true,
                "preserved_unknown_keys": preserved_unknown,
            })
        }
        Err(e) => json!({
            "written": {},
            "source": LOCALE_CONF,
            "requires_root": true,
            "error": format!("write failed (likely needs root): {}", e),
        }),
    }
}

/// Parse a `KEY="value"` or `KEY=value` file into a map.
fn read_locale_conf() -> Result<std::collections::HashMap<String, String>> {
    let s = fs::read_to_string(LOCALE_CONF).context("read locale.conf")?;
    let mut out = std::collections::HashMap::new();
    for line in s.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let k = trimmed[..eq].trim().trim_start_matches("export ").trim();
            let v = trimmed[eq + 1..].trim();
            let v = v.trim_matches('"').trim_matches('\'');
            out.insert(k.to_string(), v.to_string());
        }
    }
    Ok(out)
}

fn quote_value(v: &str) -> String {
    // Escape backslashes and double-quotes inside the value.
    let mut s = String::with_capacity(v.len());
    for ch in v.chars() {
        match ch {
            '\\' | '"' => {
                s.push('\\');
                s.push(ch);
            }
            _ => s.push(ch),
        }
    }
    s
}

/// Atomic write: write to `path.tmp` (in the same directory so the
/// rename is on the same filesystem and atomic), then rename into
/// place. Prevents leaving a half-written config file if the daemon
/// is killed mid-write (SIGKILL, OOM, power loss).
fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    // Pick a unique tmp name in the same directory so the rename
    // is atomic on POSIX (same filesystem required).
    let tmp = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("deskbrid")
    ));
    fs::write(&tmp, content)?;
    fs::rename(&tmp, path)
}

// ---------- Timezone ----------

/// Read the current timezone. Resolves /etc/localtime (realpath) and
/// cross-references /etc/timezone when present. Computes UTC offset by
/// calling `date +%z` against the system clock.
pub(super) fn timezone_get() -> Value {
    let (resolved, symlink_target) = resolve_localtime();

    let from_file = fs::read_to_string(ETC_TIMEZONE)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Prefer /etc/timezone when present (it carries the canonical name),
    // otherwise use the realpath-derived name.
    let timezone = from_file
        .clone()
        .or_else(|| resolved.clone())
        .unwrap_or_default();

    let (utc_offset_minutes, dst_active, is_utc) = compute_offset_and_dst();
    json!({
        "timezone": timezone,
        "utc_offset_minutes": utc_offset_minutes,
        "dst_active": dst_active,
        "is_utc": is_utc,
        "symlink": ETC_LOCALTIME,
        "symlink_target": symlink_target,
        "resolved_from_localtime": resolved,
        "from_etc_timezone": from_file,
    })
}

/// Set the timezone by writing /etc/timezone and re-pointing
/// /etc/localtime at /usr/share/zoneinfo/{name}. Requires root.
pub(super) fn timezone_set(timezone: &str) -> Value {
    if timezone.is_empty() {
        return json!({
            "set": false,
            "error": "timezone is empty",
            "requires_root": true,
        });
    }
    // Validate against zoneinfo dir
    let target = Path::new(ZONEINFO_DIR).join(timezone);
    if !target.exists() {
        return json!({
            "set": false,
            "error": format!("timezone {:?} not found in {}", timezone, ZONEINFO_DIR),
            "requires_root": true,
        });
    }
    // Reject obviously bad paths (must not contain .. or be absolute)
    if timezone.contains("..") || timezone.starts_with('/') {
        return json!({
            "set": false,
            "error": "timezone path traversal not allowed",
            "requires_root": true,
        });
    }

    let previous = timezone_get()
        .pointer("/timezone")
        .cloned()
        .unwrap_or(Value::Null);

    // Stage 1: write /etc/timezone atomically (tmp + rename).
    let tz_path = Path::new(ETC_TIMEZONE);
    if let Err(e) = atomic_write(tz_path, &format!("{}\n", timezone)) {
        return json!({
            "set": false,
            "previous": previous,
            "error": format!("write {} failed (likely needs root): {}", ETC_TIMEZONE, e),
            "requires_root": true,
        });
    }
    // Stage 2: create /etc/localtime as a fresh symlink in a tmp name,
    // then atomically rename it into place. If the rename fails, the
    // /etc/timezone write has already landed — that's the unavoidable
    // ordering, but at least /etc/localtime is not left in a half-state
    // (we never delete the old symlink before the new one is in place).
    let lt_path = Path::new(ETC_LOCALTIME);
    let lt_tmp = Path::new("/etc/.localtime.tmp");
    // Clean up any stale tmp from a prior failed attempt
    let _ = fs::remove_file(lt_tmp);
    let link_result = symlink(&target, lt_tmp);
    if let Err(e) = link_result {
        return json!({
            "set": false,
            "previous": previous,
            "note": format!("{} was written, but {} symlink failed", ETC_TIMEZONE, ETC_LOCALTIME),
            "error": format!("symlink {} failed (likely needs root): {}", lt_tmp.display(), e),
            "requires_root": true,
        });
    }
    match fs::rename(lt_tmp, lt_path) {
        Ok(()) => json!({
            "set": true,
            "previous": previous,
            "timezone": timezone,
            "symlink_target": target.display().to_string(),
            "requires_root": true,
        }),
        Err(e) => {
            // Roll back the staged symlink so we don't leak it
            let _ = fs::remove_file(lt_tmp);
            json!({
                "set": false,
                "previous": previous,
                "note": format!("{} was written, but {} rename failed", ETC_TIMEZONE, ETC_LOCALTIME),
                "error": format!("rename to {} failed: {}", ETC_LOCALTIME, e),
                "requires_root": true,
            })
        }
    }
}

/// Resolve /etc/localtime → /usr/share/zoneinfo/Region/City
fn resolve_localtime() -> (Option<String>, Option<String>) {
    let p = Path::new(ETC_LOCALTIME);
    let target = if p.is_symlink() {
        std::fs::read_link(p).ok()
    } else {
        // Some systems have a regular file (copied, not symlinked).
        // We can still report the path; offset is computed via `date`.
        Some(p.to_path_buf())
    };
    let resolved = target.as_ref().and_then(|t| {
        // If the resolved path lives under ZONEINFO_DIR, return its suffix
        t.canonicalize()
            .ok()
            .and_then(|canonical| canonical.to_str().map(String::from))
            .and_then(|rest| rest.strip_prefix(ZONEINFO_DIR).map(|s| s.to_string()))
            .map(|stripped| stripped.trim_start_matches('/').to_string())
    });
    let target_str = target.and_then(|t| t.to_str().map(String::from));
    (resolved, target_str)
}

/// Compute UTC offset and DST flag using `date +%z` and `date +%Z`.
/// Falls back to 0/UTC if `date` is unavailable.
fn compute_offset_and_dst() -> (i32, bool, bool) {
    let offset_out = Command::new("date").arg("+%z").output().ok();
    let name_out = Command::new("date").arg("+%Z").output().ok();
    let offset_minutes = offset_out
        .as_ref()
        .and_then(|o| String::from_utf8(o.stdout.clone()).ok())
        .and_then(|s| parse_offset(&s));
    let tz_name = name_out
        .as_ref()
        .and_then(|o| String::from_utf8(o.stdout.clone()).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let is_utc = tz_name == "UTC";
    // Heuristic: many DST tz names contain 'D' (EDT, CDT, MDT, PDT, HADT,
    // AKDT) or 'S' (BST, CEST, EEST). Winter names are shorter (ET, CT,
    // PT). We deliberately err on the side of false positives — callers
    // who care about exact DST status should use a dedicated library.
    let dst_active = !is_utc && (tz_name.contains('D') || tz_name.contains('S'));
    (offset_minutes.unwrap_or(0), dst_active, is_utc)
}

/// Parse `±HHMM` (e.g. "-0400", "+0530") into minutes.
fn parse_offset(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.len() != 5 {
        return None;
    }
    let sign = match s.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let hh: i32 = s[1..3].parse().ok()?;
    let mm: i32 = s[3..5].parse().ok()?;
    Some(sign * (hh * 60 + mm))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_offset_handles_signed_form() {
        assert_eq!(parse_offset("+0000"), Some(0));
        assert_eq!(parse_offset("-0400"), Some(-240));
        assert_eq!(parse_offset("+0530"), Some(330));
        assert_eq!(parse_offset("+1245"), Some(12 * 60 + 45));
    }

    #[test]
    fn parse_offset_rejects_garbage() {
        assert_eq!(parse_offset(""), None);
        assert_eq!(parse_offset("bad"), None);
        assert_eq!(parse_offset("+04000"), None); // too long
        assert_eq!(parse_offset("+0a00"), None); // non-digit
    }

    #[test]
    fn quote_value_escapes_quotes_and_backslashes() {
        assert_eq!(quote_value("plain"), "plain");
        assert_eq!(quote_value("with\"quote"), "with\\\"quote");
        assert_eq!(quote_value("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn read_locale_conf_handles_missing_file() {
        // /etc/locale.conf may or may not exist on the test machine;
        // both branches must return Ok (empty map or parsed map).
        let res = read_locale_conf();
        match res {
            Ok(_) => {}
            Err(e) => {
                let msg = e.to_string();
                assert!(msg.contains("read locale.conf") || msg.contains("os error 2"));
            }
        }
    }

    #[test]
    fn locale_get_returns_all_keys() {
        let v = locale_get();
        let values = v.get("values").and_then(|x| x.as_object()).unwrap();
        for k in LOCALE_KEYS {
            assert!(values.contains_key(*k), "missing key {}", k);
        }
        assert_eq!(
            v.get("set_count").and_then(|x| x.as_u64()).unwrap() as usize,
            values.values().filter(|val| !val.is_null()).count()
        );
    }
}
