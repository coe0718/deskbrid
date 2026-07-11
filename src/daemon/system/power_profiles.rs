//! Power profiles daemon integration via D-Bus.
//!
//! Talks to `net.hadess.PowerProfiles` on the system bus. Power profiles daemon
//! is shipped (or available as a flatpak/runtime dep) on GNOME, KDE Plasma 6,
//! and most desktop distributions today. If it isn't running, the action
//! gracefully reports no available profiles.
//!
//! Why system bus and not session: `power-profiles-daemon` is a privileged
//! root-owned daemon (it needs to alter kernel power settings on behalf of
//! the user), so it lives on the system bus. DE session-bus shims that wrap
//! it always proxy to the same DBus name, so callers from the user's session
//! connect directly to the system bus via `zbus::Connection::system`.

use anyhow::{Context, bail};
use serde_json::{Value, json};
use zbus::zvariant::{Array, Dict, Str, Value as ZValue};

const SERVICE: &str = "net.hadess.PowerProfiles";
const PATH: &str = "/net/hadess/PowerProfiles";
const INTERFACE: &str = "net.hadess.PowerProfiles";

/// Open a system-bus connection. The caller probes the daemon by attempting
/// its first real D-Bus call — a cheaper separate liveness check (Peer.Ping)
/// is rejected by some systemd-bus policy configurations even when the actual
/// target service is reachable.
async fn connect() -> anyhow::Result<zbus::Connection> {
    zbus::Connection::system()
        .await
        .context("failed to open system bus for power-profiles-daemon")
}

/// Read `Profiles` (aa{sv}) via `GetAll`, which returns `a{sv}` directly
/// (no variant wrapping). The variant values themselves are dicts we iterate
/// via `zvariant::Array`/`zvariant::Dict` to avoid lifetime-bound deserialization
/// of generic `HashMap<String, OwnedValue>` from a borrowed reply body.
async fn list_profiles(conn: &zbus::Connection) -> anyhow::Result<Vec<String>> {
    let reply = conn
        .call_method(
            Some(SERVICE),
            PATH,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(INTERFACE,),
        )
        .await
        .context("failed to read Profiles via GetAll")?;

    // Reply signature is `a{sv}`.
    let props: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
        reply.body().deserialize()?;

    let profiles_val = props
        .get("Profiles")
        .ok_or_else(|| anyhow::anyhow!("Profiles property missing from GetAll"))?;

    let profiles_arr = profiles_val
        .downcast_ref::<Array>()
        .map_err(|e| anyhow::anyhow!("Profiles: expected array: {e}"))?;

    let mut out: Vec<String> = Vec::with_capacity(profiles_arr.len());
    for entry in profiles_arr.iter() {
        // Each entry is a{sv} — a dict.
        let dict = entry
            .downcast_ref::<Dict>()
            .map_err(|e| anyhow::anyhow!("Profile entry: expected dict: {e}"))?;
        for (key, val) in dict.iter() {
            if let Ok(k) = key.downcast_ref::<Str>()
                && k.as_str() == "Profile"
                && let Ok(s) = val.downcast_ref::<Str>()
            {
                out.push(s.to_string());
                break;
            }
        }
    }
    Ok(out)
}

/// Read the `ActiveProfile` (s) property via GetAll — same reasoning as
/// list_profiles. Returns `Ok(None)` if the daemon hasn't picked one.
async fn active_profile(conn: &zbus::Connection) -> anyhow::Result<Option<String>> {
    let reply = conn
        .call_method(
            Some(SERVICE),
            PATH,
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(INTERFACE,),
        )
        .await
        .context("failed to read ActiveProfile via GetAll")?;

    let props: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
        reply.body().deserialize()?;
    let active = props
        .get("ActiveProfile")
        .and_then(|v| v.downcast_ref::<Str>().ok().map(|s| s.to_string()))
        .unwrap_or_default();
    Ok((!active.is_empty()).then_some(active))
}

/// Set the active profile via `org.freedesktop.DBus.Properties.Set`.
/// Caller validates the name is in the available list first.
async fn set_active(conn: &zbus::Connection, profile: &str) -> anyhow::Result<()> {
    conn.call_method(
        Some(SERVICE),
        PATH,
        Some("org.freedesktop.DBus.Properties"),
        "Set",
        &(INTERFACE, "ActiveProfile", ZValue::Str(profile.into())),
    )
    .await
    .context("failed to set ActiveProfile")?;
    Ok(())
}

/// `power.profile.list` action — just the available profile names.
/// Output: `{ "profiles": [...], "available": true }`.
pub async fn list() -> anyhow::Result<Value> {
    let conn = connect().await?;
    let profiles = list_profiles(&conn).await?;
    Ok(json!({
        "profiles": profiles,
        "available": true,
    }))
}

/// `power.profile.get` action — currently active + list.
/// `active` is `null` if the daemon hasn't picked one (rare).
pub async fn get() -> anyhow::Result<Value> {
    let conn = connect().await?;
    let profiles = list_profiles(&conn).await?;
    let active = active_profile(&conn).await?;
    Ok(json!({
        "active": active,
        "profiles": profiles,
        "available": true,
    }))
}

/// `power.profile.set` action — switch profile. Validates against the
/// available list first to fail fast with a clean error.
pub async fn set(profile: &str) -> anyhow::Result<Value> {
    if profile.is_empty() {
        bail!("profile must be non-empty");
    }
    let conn = connect().await?;
    let profiles = list_profiles(&conn).await?;
    if !profiles.iter().any(|p| p == profile) {
        bail!(
            "profile '{}' is not available; available profiles: [{}]",
            profile,
            profiles.join(", ")
        );
    }
    let previous = active_profile(&conn).await?;
    set_active(&conn, profile).await?;
    // Re-read to confirm (or report). The set is fire-and-forget but
    // surfacing the new active value lets the caller verify.
    let new_active = active_profile(&conn).await?;
    Ok(json!({
        "active": new_active,
        "previous": previous,
        "requested": profile,
    }))
}
