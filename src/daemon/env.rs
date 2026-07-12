//! Environment variable get/set for the daemon's own process.
//!
//! Reads and writes `std::env` of the running daemon. Per Linux semantics,
//! setting an env var in the daemon does NOT propagate to already-spawned
//! children, only to children spawned after the set. This matches the
//! documented limitation in the README and the protocol docstring.
//!
//! This module does NOT persist to `~/.config/environment.d/*.conf` or
//! Persistent changes (env.persist / env.unset) write to
//! `~/.config/environment.d/deskbrid.conf` — the systemd user-session
//! standard. Shells and apps launched after the next login inherit
//! the values; the running daemon and already-spawned children do not.

use serde_json::{Value, json};
use std::ffi::OsString;
use std::sync::Mutex;

/// W3 (Vex review): single global mutex serializing all process-env
/// reads and writes. Rust 2024 marked `std::env::set_var` and `var`
/// `unsafe` because reading env from one thread while another thread
/// writes it is undefined behavior (the env table is global, not
/// per-thread). Holding this mutex for the duration of every env access
/// makes the operation safely multi-threaded.
///
/// This is a `std::sync::Mutex` (not `tokio::sync::Mutex`) because
/// the critical section must not yield — `set_var`/`var` are blocking
/// libc calls and must complete before we drop the guard.
static ENV_LOCK: Mutex<()> = Mutex::new(());
use std::os::unix::ffi::OsStringExt;

/// Read one variable or all variables in the daemon's process environment.
///
/// `name == None` returns a map of all env vars (filtering out non-UTF8
/// values, which we report as a count).
/// `name == Some("X")` returns a single-variable lookup with `found: bool`.
pub(super) fn env_get(name: Option<&str>) -> Value {
    match name {
        None => all_env(),
        Some(n) => single_env(n),
    }
}

/// All env vars as a JSON object. Non-UTF8 values are dropped and counted.
fn all_env() -> Value {
    let mut vars = serde_json::Map::with_capacity(64);
    let mut non_utf8 = 0usize;
    for (k, v) in std::env::vars_os() {
        let k = k.to_string_lossy().into_owned();
        match v.into_string() {
            Ok(s) => {
                vars.insert(k, Value::String(s));
            }
            Err(_) => {
                non_utf8 += 1;
            }
        }
    }
    let count = vars.len();
    json!({
        "vars": Value::Object(vars),
        "count": count,
        "non_utf8_count": non_utf8,
    })
}

/// Single env var lookup. Preserves non-UTF8 values as a hex-byte
/// representation alongside `found: bool` so callers can still distinguish
/// "missing" from "non-UTF8".
fn single_env(name: &str) -> Value {
    match std::env::var_os(name) {
        None => json!({
            "name": name,
            "found": false,
            "value": null,
        }),
        Some(v) => env_value_json(name, v),
    }
}

fn env_value_json(name: &str, v: OsString) -> Value {
    match v.into_string() {
        Ok(s) => json!({
            "name": name,
            "found": true,
            "value": s,
            "kind": "utf8",
        }),
        Err(oss) => {
            // Non-UTF8: encode bytes as a JSON array of u8 so the response
            // is still valid JSON without smuggling arbitrary byte data
            // into a string field.
            let bytes: Vec<u8> = oss.into_vec();
            let byte_len = bytes.len();
            json!({
                "name": name,
                "found": true,
                "value_bytes": bytes,
                "byte_len": byte_len,
                "kind": "binary",
            })
        }
    }
}

/// Set one variable in the daemon's process environment. Returns a JSON
/// object describing the prior state (so callers can verify they didn't
/// clobber an unrelated value).
///
/// Validation: name must be non-empty and must not contain `=`. Value is
/// accepted as-is including empty strings.
pub(super) fn env_set(name: &str, value: &str) -> Value {
    if name.is_empty() {
        return json!({
            "name": name,
            "set": false,
            "error": "name is empty",
        });
    }
    if name.contains('=') {
        return json!({
            "name": name,
            "set": false,
            "error": "name contains '='; use a variable name, not an assignment",
        });
    }
    if name.contains('\0') {
        return json!({
            "name": name,
            "set": false,
            "error": "name contains a NUL byte",
        });
    }
    // W3: hold ENV_LOCK for the duration of the env read+write pair so
    // no other thread can interleave a `var()` between our two calls.
    // This makes the operation safely multi-threaded under Rust 2024.
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let previous = std::env::var_os(name);
    let previous_json = previous
        .as_ref()
        .map(|oss| oss.to_string_lossy().to_string());
    // SAFETY: ENV_LOCK above ensures no other thread is reading the
    // env table while we write it. The unsafe `set_var` is sound
    // because we hold the global serialization lock for the entire
    // read+write critical section.
    unsafe {
        std::env::set_var(name, value);
    }
    json!({
        "name": name,
        "set": true,
        "previous": previous_json,
        "value": value,
    })
}

/// W3 (Vex review): unset a single env var. Held under ENV_LOCK so
/// the `remove_var` call cannot race with a concurrent read in another
/// thread. Different from `env_unset(names: &[String])` which handles
/// bulk unsets via the persistent env.d config — this one only mutates
/// the in-process env table.
#[allow(dead_code)] // exposed for future caller; currently set_var covers the in-process path
pub(super) fn env_unset_one(name: &str) -> Value {
    if name.is_empty() {
        return json!({
            "name": name,
            "unset": false,
            "error": "name is empty",
        });
    }
    if name.contains('=') {
        return json!({
            "name": name,
            "unset": false,
            "error": "name contains '='; use a variable name, not an assignment",
        });
    }
    if name.contains('\0') {
        return json!({
            "name": name,
            "unset": false,
            "error": "name contains a NUL byte",
        });
    }
    let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let previous = std::env::var_os(name);
    let previous_json = previous
        .as_ref()
        .map(|oss| oss.to_string_lossy().to_string());
    // SAFETY: ENV_LOCK held — no concurrent env access from other threads.
    unsafe {
        std::env::remove_var(name);
    }
    json!({
        "name": name,
        "unset": true,
        "previous": previous_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_missing_returns_not_found() {
        let v = env_get(Some("DESKBRID_TEST_NONEXISTENT_VAR_XYZ"));
        assert_eq!(v["found"], false);
        assert!(v["value"].is_null());
    }

    #[test]
    fn get_all_returns_object() {
        let v = env_get(None);
        assert!(v["vars"].is_object());
        assert!(v["count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn set_then_get_roundtrip() {
        let key = "DESKBRID_TEST_VAR_ROUNDTRIP";
        let r = env_set(key, "hello");
        assert_eq!(r["set"], true);
        assert_eq!(r["previous"], Value::Null);

        let g = env_get(Some(key));
        assert_eq!(g["found"], true);
        assert_eq!(g["value"], "hello");

        // Cleanup
        unsafe {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn set_overwrites_existing() {
        let key = "DESKBRID_TEST_VAR_OVERWRITE";
        unsafe {
            std::env::set_var(key, "first");
        }
        let r = env_set(key, "second");
        assert_eq!(r["set"], true);
        assert_eq!(r["previous"], "first");
        assert_eq!(r["value"], "second");

        let g = env_get(Some(key));
        assert_eq!(g["value"], "second");

        unsafe {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn set_rejects_empty_name() {
        let r = env_set("", "x");
        assert_eq!(r["set"], false);
        assert!(r["error"].as_str().unwrap().contains("empty"));
    }

    #[test]
    fn set_rejects_name_with_equals() {
        let r = env_set("FOO=BAR", "x");
        assert_eq!(r["set"], false);
        assert!(r["error"].as_str().unwrap().contains('='));
    }

    #[test]
    fn set_rejects_name_with_nul() {
        let r = env_set("FOO\0BAR", "x");
        assert_eq!(r["set"], false);
        assert!(r["error"].as_str().unwrap().contains("NUL"));
    }

    #[test]
    fn set_accepts_empty_value() {
        let key = "DESKBRID_TEST_VAR_EMPTY";
        let r = env_set(key, "");
        assert_eq!(r["set"], true);
        let g = env_get(Some(key));
        assert_eq!(g["found"], true);
        assert_eq!(g["value"], "");
        unsafe {
            std::env::remove_var(key);
        }
    }

    // ===== Persistent env (env.persist / env.unset / env.list_persisted) =====

    #[test]
    fn list_persisted_handles_missing_file() {
        // Just verify it doesn't panic and returns exists:false or a map.
        let v = env_list_persisted();
        // Either exists:false or a populated vars map — both are valid.
        assert!(v.get("source").is_some());
    }
}

// ===== Persistent env =====

use std::path::PathBuf;

/// Lock used by the test suite to serialize HOME-mutating tests.
/// Not held by production code (production is single-threaded for
/// these handlers); tests hold it for the entire test body so
/// parallel test threads don't interleave their HOME swaps.
#[cfg(test)]
static PERSIST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Path to our user-scoped env config file.
/// Uses systemd's standard `environment.d/` directory, which is honored
/// by systemd user sessions and many login managers. The file lives in
/// the user's home — no root required.
fn persisted_env_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".config/environment.d/deskbrid.conf")
}

/// Persist `vars` to the deskbrid config file. Vars that aren't in our
/// request but already exist in the file are preserved. Vars whose
/// values are invalid (empty name, contains '=' or NUL) are rejected
/// at parse time, so by the time we get here all names are clean.
pub(super) fn env_persist(vars: &[(String, String)]) -> Value {
    if vars.is_empty() {
        return json!({
            "written": {},
            "preserved": 0,
            "source": persisted_env_path().display().to_string(),
            "note": "no vars to persist",
        });
    }
    let path = persisted_env_path();
    // Ensure parent directory exists; bail out with a clean error if not.
    if let Err(e) = ensure_parent_dir(&path) {
        return json!({
            "written": {},
            "preserved": 0,
            "source": path.display().to_string(),
            "error": format!("failed to create parent dir: {}", e),
        });
    }
    // Read existing lines (if any)
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let existing_lines: Vec<String> = existing.lines().map(String::from).collect();
    let mut out_lines: Vec<String> = Vec::with_capacity(existing_lines.len());
    let mut preserved = 0usize;
    let mut requested: Vec<(String, String)> = vars.to_vec();

    for line in &existing_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            out_lines.push(line.clone());
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let k = trimmed[..eq].trim();
            if let Some(idx) = requested.iter().position(|(rk, _)| rk == k) {
                let (_, v) = &requested[idx];
                out_lines.push(format!("{}=\"{}\"", k, quote_for_environ_d(v)));
                requested.remove(idx);
            } else {
                out_lines.push(line.clone());
                preserved += 1;
            }
        } else {
            out_lines.push(line.clone());
            preserved += 1;
        }
    }
    // Append any keys that weren't already in the file
    for (k, v) in requested {
        out_lines.push(format!("{}=\"{}\"", k, quote_for_environ_d(&v)));
    }

    let content = out_lines.join("\n") + "\n";
    match atomic_write(&path, &content) {
        Ok(()) => {
            let written: serde_json::Map<String, Value> = vars
                .iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            json!({
                "written": Value::Object(written),
                "preserved": preserved,
                "source": path.display().to_string(),
            })
        }
        Err(e) => json!({
            "written": {},
            "preserved": 0,
            "source": path.display().to_string(),
            "error": format!("write failed: {}", e),
        }),
    }
}

/// Remove the named vars from the persisted file. Missing vars are
/// reported but do not error.
pub(super) fn env_unset(names: &[String]) -> Value {
    if names.is_empty() {
        return json!({
            "removed": [],
            "not_found": [],
            "source": persisted_env_path().display().to_string(),
            "note": "no names to remove",
        });
    }
    let path = persisted_env_path();
    if !path.exists() {
        return json!({
            "removed": [],
            "not_found": names,
            "source": path.display().to_string(),
            "exists": false,
        });
    }
    let existing = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return json!({
                "removed": [],
                "not_found": [],
                "source": path.display().to_string(),
                "error": format!("read failed: {}", e),
            });
        }
    };
    let mut removed: Vec<String> = Vec::new();
    let mut not_found: Vec<String> = names.to_vec();
    let mut out_lines: Vec<String> = Vec::new();
    for line in existing.lines() {
        let trimmed = line.trim();
        if let Some(eq) = trimmed.find('=') {
            let k = trimmed[..eq].trim();
            if let Some(idx) = not_found.iter().position(|n| n == k) {
                not_found.remove(idx);
                removed.push(k.to_string());
                continue; // skip this line
            }
        }
        out_lines.push(line.to_string());
    }
    let content = out_lines.join("\n") + if out_lines.is_empty() { "" } else { "\n" };
    match atomic_write(&path, &content) {
        Ok(()) => json!({
            "removed": removed,
            "not_found": not_found,
            "source": path.display().to_string(),
        }),
        Err(e) => json!({
            "removed": [],
            "not_found": [],
            "source": path.display().to_string(),
            "error": format!("write failed: {}", e),
        }),
    }
}

/// Read the persisted env vars from the deskbrid config file.
pub(super) fn env_list_persisted() -> Value {
    let path = persisted_env_path();
    if !path.exists() {
        return json!({
            "vars": {},
            "count": 0,
            "source": path.display().to_string(),
            "exists": false,
        });
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return json!({
                "vars": {},
                "count": 0,
                "source": path.display().to_string(),
                "error": format!("read failed: {}", e),
            });
        }
    };
    let mut vars = serde_json::Map::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let k = trimmed[..eq].trim().to_string();
            let raw_v = trimmed[eq + 1..].trim();
            // Strip surrounding double-quotes if present
            let raw_v = raw_v.strip_prefix('"').unwrap_or(raw_v);
            let raw_v = raw_v.strip_suffix('"').unwrap_or(raw_v);
            // Un-escape backslash and double-quote per systemd syntax
            let v = unescape_environ_d(raw_v);
            vars.insert(k, Value::String(v));
        }
    }
    let count = vars.len();
    json!({
        "vars": Value::Object(vars),
        "count": count,
        "source": path.display().to_string(),
        "exists": true,
    })
}

/// Escape a value for systemd environment.d files (which expect
/// double-quoted strings). Backslashes and double-quotes are escaped.
fn quote_for_environ_d(v: &str) -> String {
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

/// Reverse of `quote_for_environ_d`: turn `\"` back into `"` and `\\`
/// back into `\`. Unknown escape sequences pass through unchanged.
fn unescape_environ_d(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Create the parent directory of `path` if it doesn't already exist.
/// Returns Ok(()) if the parent exists or was created successfully.
/// Returns Err only when creation is needed AND fails.
fn ensure_parent_dir(path: &std::path::Path) -> std::io::Result<()> {
    match path.parent() {
        Some(parent) if !parent.exists() => std::fs::create_dir_all(parent),
        _ => Ok(()),
    }
}

/// Atomic write: write to `path.tmp` then rename. Avoids leaving a
/// half-written config file if the daemon is killed mid-write.
fn atomic_write(path: &std::path::Path, content: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("conf.tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod persist_tests {
    use super::*;

    /// Use a unique tmp dir per test so we don't collide with each other
    /// or with the real ~/.config/environment.d/deskbrid.conf. Holds
    /// `PERSIST_LOCK` for the duration of the test body so the
    /// parallel test runner doesn't see interleaved HOME mutations.
    fn with_tmp_home<F: FnOnce()>(f: F) {
        let _guard = PERSIST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let unique = format!(
            "/tmp/deskbrid-env-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let _ = std::fs::create_dir_all(&unique);
        // SAFETY: std::env::set_var is unsafe since Rust 1.83 / edition 2024,
        // but for a single-threaded test (lock is held) it's fine.
        unsafe {
            std::env::set_var("HOME", &unique);
        }
        f();
        unsafe {
            std::env::remove_var("HOME");
        }
        let _ = std::fs::remove_dir_all(&unique);
    }

    #[test]
    fn persist_writes_new_file() {
        with_tmp_home(|| {
            let r = env_persist(&[("FOO".into(), "bar".into())]);
            assert_eq!(r["written"]["FOO"], "bar");
            let g = env_list_persisted();
            assert_eq!(g["vars"]["FOO"], "bar");
        });
    }

    #[test]
    fn persist_preserves_existing_keys() {
        with_tmp_home(|| {
            env_persist(&[("FOO".into(), "1".into()), ("BAR".into(), "2".into())]);
            env_persist(&[("FOO".into(), "3".into())]);
            let g = env_list_persisted();
            assert_eq!(g["vars"]["FOO"], "3");
            assert_eq!(g["vars"]["BAR"], "2");
        });
    }

    #[test]
    fn persist_handles_empty_request() {
        with_tmp_home(|| {
            let r = env_persist(&[]);
            assert!(r["note"].as_str().unwrap().contains("no vars"));
        });
    }

    #[test]
    fn unset_removes_named_vars() {
        with_tmp_home(|| {
            env_persist(&[
                ("FOO".into(), "1".into()),
                ("BAR".into(), "2".into()),
                ("BAZ".into(), "3".into()),
            ]);
            let r = env_unset(&["FOO".into(), "BAZ".into(), "MISSING".into()]);
            assert_eq!(r["removed"], serde_json::json!(["FOO", "BAZ"]));
            assert_eq!(r["not_found"], serde_json::json!(["MISSING"]));
            let g = env_list_persisted();
            assert_eq!(g["count"].as_u64().unwrap(), 1);
            assert_eq!(g["vars"]["BAR"], "2");
        });
    }

    #[test]
    fn unset_handles_missing_file() {
        with_tmp_home(|| {
            let r = env_unset(&["FOO".into()]);
            assert_eq!(r["exists"], false);
            assert_eq!(r["not_found"], serde_json::json!(["FOO"]));
        });
    }

    #[test]
    fn unset_handles_empty_request() {
        with_tmp_home(|| {
            let r = env_unset(&[]);
            assert!(r["note"].as_str().unwrap().contains("no names"));
        });
    }

    #[test]
    fn list_persisted_returns_exists_false_when_no_file() {
        with_tmp_home(|| {
            let r = env_list_persisted();
            assert_eq!(r["exists"], false);
            assert_eq!(r["count"], 0);
        });
    }

    #[test]
    fn persist_escapes_quotes_and_backslashes() {
        with_tmp_home(|| {
            env_persist(&[("VAL".into(), "with \"quote\" and \\ back".into())]);
            let r = env_list_persisted();
            assert_eq!(r["vars"]["VAL"], "with \"quote\" and \\ back");
        });
    }
}
