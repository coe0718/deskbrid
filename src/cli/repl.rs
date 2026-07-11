//! Interactive REPL for exploring the deskbrid daemon.
//!
//! Provides a rustyline-powered shell that lets users type action names with
//! simple `--key value` and `key=value` arguments, dispatches them through the
//! existing daemon client, and pretty-prints responses. Designed for
//! discoverability and quick experiments against a running daemon — not as a
//! scripting environment.
//!
//! Built-in commands:
//!
//! - `help` / `?`                 — show usage / list built-ins
//! - `actions [prefix]`           — list matching public action types
//! - `describe <action>`          — show the parameter names this REPL recognises for an action
//! - `set <key>=<value>`          — toggle REPL session flags (`dry_run=true` etc.)
//! - `unset <key>`                — clear a REPL session flag
//! - `state`                      — show current REPL session flags
//! - `exit` / `quit` / `:q`       — leave the REPL
//!
//! Anything else is treated as `<action> [--k v | k=v]...` and sent to the daemon.

use std::borrow::Cow;

use anyhow::Result;
use colored::Colorize;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Editor, Helper};

use crate::client;
use crate::protocol::{Action, RequestOptions};

/// Action parameter conventions understood by the REPL. Maps a few common
/// shortcut names to the canonical wire-protocol field, so users don't have
/// to memorise the exact spelling used by the daemon.
fn normalize_param<'a>(action: &str, key: &'a str) -> &'a str {
    // The daemon uses `id` for both window IDs and agent IDs; the REPL
    // accepts either form transparently. More aliases can be added here as
    // they come up in user feedback.
    let key = key.trim_start_matches('-');
    let lower = key.to_ascii_lowercase();
    match (action, lower.as_str()) {
        ("windows.focus", "window") | ("windows.focus", "window_id") => "id",
        ("windows.close", "window") | ("windows.close", "window_id") => "id",
        ("windows.get", "window") | ("windows.get", "window_id") => "id",
        _ => key,
    }
}

/// Per-session REPL state. Currently just request options, but could grow to
/// include a named default session, recent-result history, etc.
#[derive(Default, Clone)]
struct ReplState {
    options: RequestOptions,
}

impl ReplState {
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key.to_ascii_lowercase().as_str() {
            "dry_run" | "dryrun" => {
                self.options.dry_run = parse_bool(value)?;
            }
            "timeout_ms" | "timeout" => {
                self.options.timeout_ms = Some(value.parse().map_err(|_| {
                    anyhow::anyhow!("timeout_ms must be an integer (got {:?})", value)
                })?);
            }
            "confirmation" | "require_confirmation" => {
                self.options.require_confirmation = Some(parse_bool(value)?);
            }
            other => anyhow::bail!(
                "unknown REPL setting {:?} — supported: dry_run, timeout_ms, confirmation",
                other
            ),
        }
        Ok(())
    }

    fn unset(&mut self, key: &str) {
        match key.to_ascii_lowercase().as_str() {
            "dry_run" | "dryrun" => self.options.dry_run = false,
            "timeout_ms" | "timeout" => self.options.timeout_ms = None,
            "confirmation" | "require_confirmation" => self.options.require_confirmation = None,
            _ => {}
        }
    }
}

fn parse_bool(s: &str) -> Result<bool> {
    match s.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" | "" => Ok(false),
        other => anyhow::bail!("expected boolean (true/false), got {:?}", other),
    }
}

/// Auto-completer backed by the canonical public action-type list. Offers
/// matches on both action names and `--key` flags derived from a small
/// hand-curated hint map.
struct ReplCompleter;

impl Completer for ReplCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let mut candidates = Vec::new();
        let trimmed = line.trim_start();
        let leading_ws = line.len() - trimmed.len();
        let cursor_in_trimmed = pos.saturating_sub(leading_ws);

        // Completing an action name (first whitespace-delimited token)
        if !trimmed.contains(char::is_whitespace) {
            let prefix = &trimmed[..cursor_in_trimmed];
            for action in Action::public_action_types() {
                if action.starts_with(prefix) {
                    candidates.push(Pair {
                        display: action.to_string(),
                        replacement: action.to_string(),
                    });
                }
            }
            return Ok((leading_ws, candidates));
        }

        // Completing a flag inside an action invocation
        let first_space = trimmed.find(char::is_whitespace).unwrap_or(0);
        let action = &trimmed[..first_space];
        let after_action = &trimmed[first_space..];
        let cursor_in_after = cursor_in_trimmed.saturating_sub(first_space);

        // Find the current partial token after the last whitespace
        let last_ws = after_action[..cursor_in_after]
            .rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);
        let partial = &after_action[last_ws..cursor_in_after];

        if let Some(needle) = partial.strip_prefix("--") {
            for hint in flag_hints(action) {
                if hint.starts_with(needle) {
                    candidates.push(Pair {
                        display: format!("--{}", hint),
                        replacement: format!("--{}", hint),
                    });
                }
            }
        } else if partial.starts_with('-') && partial.len() == 1 {
            for hint in flag_hints(action) {
                candidates.push(Pair {
                    display: format!("--{}", hint),
                    replacement: format!("--{}", hint),
                });
            }
        }

        let replacement_start = leading_ws + first_space + last_ws;
        Ok((replacement_start, candidates))
    }
}

/// Small per-action flag hints used by the completer. We don't try to cover
/// every parameter for every action — that's what `describe` is for. These
/// are just the flags users most often reach for first.
fn flag_hints(action: &str) -> &'static [&'static str] {
    match action {
        "windows.focus" | "windows.close" | "windows.get" => &["id"],
        "windows.move_resize" => &["id", "x", "y", "w", "h"],
        "windows.activate_or_launch" => &["app", "args"],
        "screenshot" => &["monitor", "path", "format"],
        "screenshot.ocr" => &["monitor", "language"],
        "screenshot.diff" => &["a", "b", "path"],
        "region_watch.create" => &["name", "monitor", "x", "y", "w", "h", "interval_ms"],
        "region_watch.update" => &["name"],
        "region_watch.remove" => &["name"],
        "text_watch.create" => &["name", "x", "y", "w", "h"],
        "input.mouse" => &["x", "y", "button"],
        "input.keyboard" => &["keys"],
        "mpris.control" => &["player", "action"],
        "monitor.set_resolution" => &["name", "w", "h"],
        "monitor.set_rotation" => &["name", "rotation"],
        "monitor.enable" | "monitor.disable" => &["name"],
        "audio.set_volume" => &["sink", "volume"],
        "audio.set_sink_volume" => &["sink", "volume"],
        "process.start" => &["cmd", "args", "cwd"],
        "process.signal" => &["pid", "signal"],
        "service.start" | "service.stop" | "service.restart" | "service.enable"
        | "service.disable" => &["name"],
        "terminal.create" => &["shell", "cwd", "cols", "rows"],
        "terminal.write" => &["id", "data"],
        "terminal.resize" => &["id", "cols", "rows"],
        "files.read" => &["path"],
        "files.write" => &["path", "content"],
        "files.list" => &["path"],
        "files.search" => &["path", "pattern"],
        _ => &[],
    }
}

impl Hinter for ReplCompleter {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Highlighter for ReplCompleter {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(hint.bright_black().to_string())
    }
}

impl Validator for ReplCompleter {}
impl Helper for ReplCompleter {}

const BANNER: &str = r"
deskbrid REPL — type `help` for usage, `actions` to list all actions, `exit` to quit.
hint: actions accept `--key value` or `key=value` pairs, e.g. `windows.focus --id 42`.
";

fn print_help() {
    println!(
        "\
{title}

{usage}
  <action> [--key value | key=value]...    dispatch an action to the daemon
  help                                     show this help
  ?                                        alias for help
  actions [prefix]                         list public actions (optionally filtered)
  describe <action>                        show hints for <action>
  set <key>=<value>                        set REPL option (dry_run, timeout_ms, confirmation)
  unset <key>                              clear REPL option
  state                                    show current REPL options
  exit | quit | :q                         leave the REPL

{examples}
  windows.list
  windows.focus --id 42
  screenshot --monitor 0 --path /tmp/desk.png
  set dry_run=true
  unset dry_run
",
        title = "Built-in commands".bold().underline(),
        usage = "Action syntax".bold().underline(),
        examples = "Examples".bold().underline(),
    );
}

fn print_actions(prefix: Option<&str>) {
    let prefix = prefix.unwrap_or("");
    let mut shown = 0usize;
    for action in Action::public_action_types() {
        if !prefix.is_empty() && !action.starts_with(prefix) {
            continue;
        }
        println!("  {}", action.cyan());
        shown += 1;
    }
    if shown == 0 {
        println!(
            "{} no actions match {:?} (use `actions` to list all)",
            "!".yellow(),
            prefix
        );
    } else {
        println!(
            "\n{} {} action(s){}",
            "·".bright_black(),
            shown.to_string().bold(),
            if prefix.is_empty() {
                String::new()
            } else {
                format!(" matching {:?}", prefix)
            }
        );
    }
}

fn print_describe(action: &str) {
    let hints = flag_hints(action);
    if hints.is_empty() {
        println!(
            "{} no specific hints for {:?}; any well-formed daemon parameter is accepted",
            "·".bright_black(),
            action
        );
        return;
    }
    println!("Recognised flags for {}:", action.cyan());
    for hint in hints {
        println!("  --{}", hint);
    }
}

fn print_state(state: &ReplState) {
    println!(
        "{} dry_run={}  timeout_ms={:?}  confirmation={:?}",
        "·".bright_black(),
        state.options.dry_run,
        state.options.timeout_ms,
        state.options.require_confirmation,
    );
}

/// Parse a single command line into either a builtin command or an action
/// invocation. Returns `None` for empty input.
#[derive(Debug)]
enum Parsed {
    Builtin(Builtin),
    Action {
        name: String,
        data: serde_json::Value,
    },
}

#[derive(Debug)]
enum Builtin {
    Help,
    Actions(Option<String>),
    Describe(String),
    Set { key: String, value: String },
    Unset(String),
    State,
    Exit,
}

fn parse_line(line: &str) -> Result<Option<Parsed>> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    // Builtins that don't fit the action parser
    match line {
        "help" | "?" => return Ok(Some(Parsed::Builtin(Builtin::Help))),
        "exit" | "quit" | ":q" => return Ok(Some(Parsed::Builtin(Builtin::Exit))),
        "state" => return Ok(Some(Parsed::Builtin(Builtin::State))),
        _ => {}
    }

    if let Some(rest) = line.strip_prefix("actions") {
        let rest = rest.trim();
        return Ok(Some(Parsed::Builtin(Builtin::Actions(
            if rest.is_empty() {
                None
            } else {
                Some(rest.to_string())
            },
        ))));
    }

    if let Some(rest) = line.strip_prefix("describe ") {
        let action = rest.trim();
        if action.is_empty() {
            anyhow::bail!("usage: describe <action>");
        }
        return Ok(Some(Parsed::Builtin(Builtin::Describe(action.to_string()))));
    }
    if let Some(rest) = line.strip_prefix("set ") {
        let rest = rest.trim();
        let (key, value) = rest
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("usage: set <key>=<value>"))?;
        return Ok(Some(Parsed::Builtin(Builtin::Set {
            key: key.trim().to_string(),
            value: value.trim().to_string(),
        })));
    }
    if let Some(rest) = line.strip_prefix("unset ") {
        let key = rest.trim();
        if key.is_empty() {
            anyhow::bail!("usage: unset <key>");
        }
        return Ok(Some(Parsed::Builtin(Builtin::Unset(key.to_string()))));
    }

    // Otherwise: <action> [--k v | k=v]...
    let mut parts = line.split_whitespace();
    let action = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty input"))?
        .to_string();
    let mut data = serde_json::Map::new();
    let mut iter = parts.peekable();
    while let Some(tok) = iter.next() {
        let (raw_key, raw_value): (String, Option<String>) =
            if let Some(rest) = tok.strip_prefix("--") {
                if let Some((k, v)) = rest.split_once('=') {
                    (k.to_string(), Some(v.to_string()))
                } else {
                    (rest.to_string(), None)
                }
            } else if let Some(rest) = tok.strip_prefix('-') {
                if let Some((k, v)) = rest.split_once('=') {
                    (k.to_string(), Some(v.to_string()))
                } else {
                    (rest.to_string(), None)
                }
            } else if let Some((k, v)) = tok.split_once('=') {
                (k.to_string(), Some(v.to_string()))
            } else {
                // Bare positional value — REPL doesn't model positional args, but
                // be permissive: store under the action's `positional` slot.
                (
                    "positional".to_string(),
                    Some(tok.trim_matches('"').to_string()),
                )
            };

        let value: String = match raw_value {
            Some(v) => v,
            None => iter
                .next()
                .ok_or_else(|| anyhow::anyhow!("flag --{} needs a value", raw_key))?
                .to_string(),
        };

        let key = normalize_param(&action, &raw_key).to_string();
        // Try to parse as JSON value (numbers, bools); fall back to string.
        let parsed_value = if let Ok(n) = value.parse::<i64>() {
            serde_json::Value::Number(n.into())
        } else if let Ok(f) = value.parse::<f64>() {
            serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or_else(|| serde_json::Value::String(value.clone()))
        } else if value == "true" {
            serde_json::Value::Bool(true)
        } else if value == "false" {
            serde_json::Value::Bool(false)
        } else if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&value) {
            parsed
        } else {
            serde_json::Value::String(value)
        };
        data.insert(key, parsed_value);
    }

    Ok(Some(Parsed::Action {
        name: action,
        data: serde_json::Value::Object(data),
    }))
}

fn pretty_print_response(response: serde_json::Value) -> Result<()> {
    let status = response
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if status == "error" {
        let code = response
            .pointer("/error/code")
            .and_then(|v| v.as_str())
            .unwrap_or("ERROR");
        let message = response
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("(no message)");
        eprintln!("{} {}: {}", "✗".red().bold(), code.red(), message);
        return Ok(());
    }

    if let Some(data) = response.get("data") {
        if data.is_null() {
            println!("{} ok", "✓".green().bold());
        } else {
            println!(
                "{} {}\n{}",
                "✓".green().bold(),
                "ok".dimmed(),
                serde_json::to_string_pretty(data)?.cyan()
            );
        }
    } else {
        println!(
            "{} {}",
            "✓".green().bold(),
            serde_json::to_string_pretty(&response)?.cyan()
        );
    }
    Ok(())
}

/// Entry point invoked from `main.rs`. The `sock` argument is reserved for
/// future use — currently the socket path is read from `XDG_RUNTIME_DIR` via
/// `client::socket_path()`. It's accepted here so the CLI surface can grow
/// without breaking callers.
pub async fn run(_sock: Option<String>) -> Result<()> {
    println!("{}", BANNER.trim_start().bright_blue());

    let mut rl = Editor::new()?;
    rl.set_helper(Some(ReplCompleter));
    let history_path = history_path();
    if let Some(path) = &history_path {
        let _ = rl.load_history(path);
    }

    let mut state = ReplState::default();

    loop {
        let prompt = format!("{} ", "deskbrid>".bright_green().bold());
        let line = match rl.readline(&prompt) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!("(Ctrl-C — type `exit` to leave)");
                continue;
            }
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                eprintln!("readline error: {}", err);
                break;
            }
        };

        let _ = rl.add_history_entry(line.as_str());

        match parse_line(&line) {
            Ok(Some(Parsed::Builtin(Builtin::Exit))) => break,
            Ok(Some(Parsed::Builtin(Builtin::Help))) => print_help(),
            Ok(Some(Parsed::Builtin(Builtin::Actions(prefix)))) => print_actions(prefix.as_deref()),
            Ok(Some(Parsed::Builtin(Builtin::Describe(action)))) => print_describe(&action),
            Ok(Some(Parsed::Builtin(Builtin::Set { key, value }))) => {
                if let Err(e) = state.set(&key, &value) {
                    eprintln!("{} {}", "✗".red().bold(), e);
                }
            }
            Ok(Some(Parsed::Builtin(Builtin::Unset(key)))) => state.unset(&key),
            Ok(Some(Parsed::Builtin(Builtin::State))) => print_state(&state),
            Ok(Some(Parsed::Action { name, data })) => {
                match client::send_raw(&name, data, state.options.clone()).await {
                    Ok(response) => {
                        if let Err(e) = pretty_print_response(response) {
                            eprintln!("{} {}", "✗".red().bold(), e);
                        }
                    }
                    Err(e) => {
                        eprintln!("{} {}", "✗".red().bold(), e);
                    }
                }
            }
            Ok(None) => {}
            Err(e) => eprintln!("{} {}", "✗".red().bold(), e),
        }
    }

    if let Some(path) = history_path {
        let _ = rl.save_history(&path);
    }
    println!("bye.");
    Ok(())
}

/// Persistent history file location. We deliberately use `dirs` so the file
/// lives next to other user shell history (typically `~/.local/share/deskbrid/repl_history`).
fn history_path() -> Option<std::path::PathBuf> {
    let base = dirs::data_local_dir()?;
    let dir = base.join("deskbrid");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("repl_history"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_returns_none() {
        assert!(parse_line("").unwrap().is_none());
        assert!(parse_line("   ").unwrap().is_none());
    }

    #[test]
    fn parse_builtin_help() {
        matches!(
            parse_line("help").unwrap(),
            Some(Parsed::Builtin(Builtin::Help))
        );
    }

    #[test]
    fn parse_builtin_exit() {
        for variant in ["exit", "quit", ":q"] {
            assert!(matches!(
                parse_line(variant).unwrap(),
                Some(Parsed::Builtin(Builtin::Exit))
            ));
        }
    }

    #[test]
    fn parse_action_no_args() {
        match parse_line("windows.list").unwrap() {
            Some(Parsed::Action { name, data }) => {
                assert_eq!(name, "windows.list");
                assert_eq!(data.as_object().unwrap().len(), 0);
            }
            other => panic!("expected action, got {:?}", other),
        }
    }

    #[test]
    fn parse_action_with_flags() {
        match parse_line("windows.focus --id 42").unwrap() {
            Some(Parsed::Action { name, data }) => {
                assert_eq!(name, "windows.focus");
                assert_eq!(data["id"], 42);
            }
            other => panic!("expected action, got {:?}", other),
        }
    }

    #[test]
    fn parse_action_with_eq_syntax() {
        match parse_line("screenshot monitor=0 path=/tmp/x.png").unwrap() {
            Some(Parsed::Action { name, data }) => {
                assert_eq!(name, "screenshot");
                assert_eq!(data["monitor"], 0);
                assert_eq!(data["path"], "/tmp/x.png");
            }
            other => panic!("expected action, got {:?}", other),
        }
    }

    #[test]
    fn parse_action_with_inline_eq() {
        match parse_line("windows.focus --id=99").unwrap() {
            Some(Parsed::Action { name, data }) => {
                assert_eq!(name, "windows.focus");
                assert_eq!(data["id"], 99);
            }
            other => panic!("expected action, got {:?}", other),
        }
    }

    #[test]
    fn parse_action_bool_value() {
        match parse_line("audio.mute --sink foo --mute true").unwrap() {
            Some(Parsed::Action { name, data }) => {
                assert_eq!(name, "audio.mute");
                assert_eq!(data["mute"], true);
            }
            other => panic!("expected action, got {:?}", other),
        }
    }

    #[test]
    fn parse_action_json_value() {
        match parse_line("terminal.write id=t1 data=[\"ls\",\"-la\"]").unwrap() {
            Some(Parsed::Action { name, data }) => {
                assert_eq!(name, "terminal.write");
                assert!(data["data"].is_array());
            }
            other => panic!("expected action, got {:?}", other),
        }
    }

    #[test]
    fn parse_set_command() {
        match parse_line("set dry_run=true").unwrap() {
            Some(Parsed::Builtin(Builtin::Set { key, value })) => {
                assert_eq!(key, "dry_run");
                assert_eq!(value, "true");
            }
            other => panic!("expected set, got {:?}", other),
        }
    }

    #[test]
    fn parse_set_requires_equals() {
        assert!(parse_line("set dry_run true").is_err());
    }

    #[test]
    fn parse_actions_with_prefix() {
        match parse_line("actions window").unwrap() {
            Some(Parsed::Builtin(Builtin::Actions(Some(prefix)))) => {
                assert_eq!(prefix, "window");
            }
            other => panic!("expected actions, got {:?}", other),
        }
    }

    #[test]
    fn normalize_param_aliases() {
        assert_eq!(normalize_param("windows.focus", "window"), "id");
        assert_eq!(normalize_param("windows.focus", "id"), "id");
        assert_eq!(normalize_param("screenshot", "monitor"), "monitor");
    }

    #[test]
    fn repl_state_set_unset() {
        let mut s = ReplState::default();
        s.set("dry_run", "true").unwrap();
        assert!(s.options.dry_run);
        s.unset("dry_run");
        assert!(!s.options.dry_run);

        s.set("timeout_ms", "1500").unwrap();
        assert_eq!(s.options.timeout_ms, Some(1500));
        s.unset("timeout");
        assert!(s.options.timeout_ms.is_none());
    }

    #[test]
    fn repl_state_unknown_setting_errors() {
        let mut s = ReplState::default();
        assert!(s.set("nonexistent", "x").is_err());
    }

    #[test]
    fn flag_hints_cover_known_actions() {
        assert!(!flag_hints("windows.focus").is_empty());
        assert!(!flag_hints("screenshot").is_empty());
        assert!(!flag_hints("terminal.write").is_empty());
    }

    #[test]
    fn pretty_print_data_and_error() {
        let ok = serde_json::json!({"status": "ok", "data": {"x": 1}});
        pretty_print_response(ok).unwrap();
        let err = serde_json::json!({
            "status": "error",
            "error": {"code": "BAD", "message": "nope"}
        });
        pretty_print_response(err).unwrap();
    }
}
