//! Speech / Text-to-Speech — `speech.speak`, `speech.stop`, `speech.voices`.
//!
//! Engine resolution (DE-agnostic, no `DesktopBackend` trait changes):
//!   - `speech.speak`  — prefer `spd-say` (speech-dispatcher, the canonical
//!     Linux TTS used by Orca/GNOME); fall back to `espeak-ng`; `engine`
//!     param can force either. Children are tracked in a registry so
//!     `speech.stop` can kill exactly what we started.
//!   - `speech.stop`   — kill all tracked children; also sends `spd-say --cancel`
//!     when the speech-dispatcher engine was in play.
//!   - `speech.voices` — list available voices (parsed from `espeak-ng --voices`,
//!     or the standard speech-dispatcher voice types when only spd-say exists).

use crate::DaemonState;
use crate::backend::DesktopBackend;
use crate::protocol::Action;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Instant;
use tokio::process::Child;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Active speech children keyed by speech_id. `speech.stop` drains this.
static ACTIVE_SPEECH: OnceLock<Mutex<HashMap<String, Child>>> = OnceLock::new();

fn active_speech() -> &'static Mutex<HashMap<String, Child>> {
    ACTIVE_SPEECH.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Is `cmd` on PATH?
async fn in_path(cmd: &str) -> bool {
    tokio::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolve which engine to use for a speak request.
async fn resolve_engine(requested: &Option<String>) -> anyhow::Result<&'static str> {
    match requested.as_deref() {
        Some("spd-say") if in_path("spd-say").await => Ok("spd-say"),
        Some("spd-say") => {
            anyhow::bail!("speech.speak: engine 'spd-say' requested but not found on PATH")
        }
        Some("espeak-ng") if in_path("espeak-ng").await => Ok("espeak-ng"),
        Some("espeak-ng") => {
            anyhow::bail!("speech.speak: engine 'espeak-ng' requested but not found on PATH")
        }
        Some(other) => anyhow::bail!(
            "speech.speak: unknown engine '{other}' (use 'spd-say', 'espeak-ng', or 'auto')"
        ),
        None => {
            if in_path("spd-say").await {
                Ok("spd-say")
            } else if in_path("espeak-ng").await {
                Ok("espeak-ng")
            } else {
                anyhow::bail!(
                    "speech.speak: no TTS engine found. Install speech-dispatcher (spd-say) or espeak-ng: sudo apt install speech-dispatcher espeak-ng"
                )
            }
        }
    }
}

fn build_spd_say_args(
    text: &str,
    voice: &Option<String>,
    rate: &Option<i32>,
    pitch: &Option<i32>,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(v) = voice {
        args.push("-t".into());
        args.push(v.clone());
    }
    if let Some(r) = rate {
        args.push("-r".into());
        args.push(r.to_string());
    }
    if let Some(p) = pitch {
        args.push("-p".into());
        args.push(p.to_string());
    }
    args.push(text.into());
    args
}

fn build_espeak_args(
    text: &str,
    voice: &Option<String>,
    rate: &Option<i32>,
    pitch: &Option<i32>,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(v) = voice {
        args.push("-v".into());
        args.push(v.clone());
    }
    if let Some(r) = rate {
        args.push("-s".into());
        args.push(r.to_string());
    }
    if let Some(p) = pitch {
        args.push("-p".into());
        args.push(p.to_string());
    }
    args.push(text.into());
    args
}

pub(crate) async fn execute_speech(
    action: Action,
    _backend: &dyn DesktopBackend,
    _state: &DaemonState,
) -> anyhow::Result<Value> {
    use Action::*;
    Ok(match action {
        SpeechSpeak {
            text,
            voice,
            rate,
            pitch,
            engine,
            wait,
        } => {
            if text.trim().is_empty() {
                anyhow::bail!("speech.speak: 'text' is required and cannot be empty");
            }

            let engine = resolve_engine(&engine).await?;
            let (bin, args) = match engine {
                "spd-say" => ("spd-say", build_spd_say_args(&text, &voice, &rate, &pitch)),
                _ => ("espeak-ng", build_espeak_args(&text, &voice, &rate, &pitch)),
            };

            let started = Instant::now();
            let mut child = tokio::process::Command::new(bin)
                .args(&args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map_err(|e| anyhow::anyhow!("speech.speak: failed to spawn {bin}: {e}"))?;

            let speech_id = Uuid::new_v4().to_string();
            let pid = child.id().unwrap_or(0);

            if wait {
                // Block until the utterance finishes.
                let _ = child.wait().await;
                serde_json::json!({
                    "speech_id": speech_id,
                    "engine": engine,
                    "pid": pid,
                    "spoken": true,
                    "duration_ms": started.elapsed().as_millis() as u64,
                })
            } else {
                // Track the child so speech.stop can cancel it.
                active_speech()
                    .lock()
                    .await
                    .insert(speech_id.clone(), child);
                serde_json::json!({
                    "speech_id": speech_id,
                    "engine": engine,
                    "pid": pid,
                    "spoken": false,
                    "note": "Use speech.stop to cancel",
                })
            }
        }
        SpeechStop => {
            let mut registry = active_speech().lock().await;
            let mut stopped = 0usize;
            for (_id, mut child) in registry.drain() {
                if child.kill().await.is_ok() {
                    stopped += 1;
                }
            }
            // Also ask speech-dispatcher to stop anything else we started
            // (spd-say routes through the dispatcher daemon).
            if in_path("spd-say").await {
                let _ = tokio::process::Command::new("spd-say")
                    .arg("--cancel")
                    .status()
                    .await;
            }
            serde_json::json!({"stopped": stopped})
        }
        SpeechListVoices => {
            // Prefer espeak-ng's real voice table; fall back to the standard
            // speech-dispatcher voice types.
            if in_path("espeak-ng").await {
                let out = tokio::process::Command::new("espeak-ng")
                    .arg("--voices")
                    .output()
                    .await;
                if let Ok(o) = out
                    && o.status.success()
                {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    let mut voices: Vec<serde_json::Value> = Vec::new();
                    for line in stdout.lines().skip(1) {
                        // Format: (language)(type)(name)(identifier)... whitespace separated
                        let cols: Vec<&str> = line.split_whitespace().collect();
                        if cols.len() >= 4 {
                            voices.push(serde_json::json!({
                                "language": cols[0],
                                "type": cols[1],
                                "name": cols[2],
                                "identifier": cols[3],
                            }));
                        }
                    }
                    serde_json::json!({"engine": "espeak-ng", "voices": voices, "count": voices.len()})
                } else {
                    serde_json::json!({"engine": "espeak-ng", "voices": [], "count": 0, "error": "espeak-ng --voices failed"})
                }
            } else if in_path("spd-say").await {
                // speech-dispatcher uses fixed voice types via `-t`
                let types = [
                    "male1",
                    "male2",
                    "male3",
                    "female1",
                    "female2",
                    "female3",
                    "child_male",
                    "child_female",
                ];
                let voices: Vec<serde_json::Value> = types
                    .iter()
                    .map(|t| serde_json::json!({"voice_type": t, "engine": "spd-say", "usage": "-t <type>"}))
                    .collect();
                serde_json::json!({"engine": "spd-say", "voices": voices, "count": voices.len()})
            } else {
                anyhow::bail!(
                    "speech.voices: no TTS engine found. Install speech-dispatcher (spd-say) or espeak-ng"
                )
            }
        }
        _ => anyhow::bail!("internal dispatch error: not a speech action"),
    })
}
