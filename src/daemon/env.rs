//! Environment variable get/set for the daemon's own process.
//!
//! Reads and writes `std::env` of the running daemon. Per Linux semantics,
//! setting an env var in the daemon does NOT propagate to already-spawned
//! children, only to children spawned after the set. This matches the
//! documented limitation in the README and the protocol docstring.
//!
//! This module does NOT persist to `~/.config/environment.d/*.conf` or
//! `/etc/environment`. Persistent changes need a separate action
//! (`env.persist` — not yet implemented; see ROADMAP #116).

use serde_json::{Value, json};
use std::ffi::OsString;
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
    let previous = std::env::var(name).ok();
    // SAFETY: std::env::set_var is `unsafe` since Rust 1.83 / edition 2024.
    // Single-threaded access to the env table is not guaranteed in async
    // contexts, but in practice this is fine for our use case (rare,
    // human-driven calls; the daemon does not concurrently set env vars).
    unsafe {
        std::env::set_var(name, value);
    }
    json!({
        "name": name,
        "set": true,
        "previous": previous,
        "value": value,
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
}
