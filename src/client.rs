use crate::protocol::{Action, RequestOptions};
use anyhow::Context;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

fn socket_path() -> String {
    std::env::var("XDG_RUNTIME_DIR")
        .map(|d| format!("{}/deskbrid.sock", d))
        .expect("XDG_RUNTIME_DIR must be set — cannot locate daemon socket")
}

/// Check if TCP transport is configured via environment variables.
fn tcp_addr() -> Option<String> {
    let host = std::env::var("DESKBRID_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("DESKBRID_PORT").ok()?;
    Some(format!("{}:{}", host, port))
}

fn tcp_token() -> Option<String> {
    std::env::var("DESKBRID_TCP_TOKEN").ok()
}

/// Connect to the daemon, send a one-shot action, and print the response.
pub async fn send_one_shot(action: Action) -> anyhow::Result<()> {
    send_one_shot_with_options(action, RequestOptions::default()).await
}

/// Connect to the daemon, send a one-shot action with request options, and print the response.
pub async fn send_one_shot_with_options(
    action: Action,
    options: RequestOptions,
) -> anyhow::Result<()> {
    if let (Some(addr), Some(token)) = (tcp_addr(), tcp_token()) {
        return send_one_shot_tcp(action, options, &addr, &token).await;
    }
    send_one_shot_unix(action, options).await
}

async fn send_one_shot_unix(action: Action, options: RequestOptions) -> anyhow::Result<()> {
    let sock = socket_path();
    let stream = UnixStream::connect(&sock).await.context(format!(
        "cannot connect to daemon at {}. Is deskbrid running?",
        sock
    ))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    run_one_shot(&mut reader, &mut writer, action, options).await
}

async fn send_one_shot_tcp(
    action: Action,
    options: RequestOptions,
    addr: &str,
    token: &str,
) -> anyhow::Result<()> {
    let stream = tokio::net::TcpStream::connect(addr)
        .await
        .context(format!("cannot connect to daemon at {}", addr))?;

    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    // Send auth message
    let auth = serde_json::json!({"type": "auth", "token": token});
    writer
        .write_all(format!("{}\n", serde_json::to_string(&auth)?).as_bytes())
        .await?;

    // Read auth response
    let mut auth_response = String::new();
    reader.read_line(&mut auth_response).await?;

    // If the daemon sends an error, propagate it
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&auth_response)
        && parsed.get("status") == Some(&serde_json::Value::String("error".into()))
    {
        let msg = parsed["error"]["message"]
            .as_str()
            .unwrap_or("authentication failed");
        anyhow::bail!("TCP auth failed: {}", msg);
    }

    run_one_shot(&mut reader, &mut writer, action, options).await
}

async fn run_one_shot<R: tokio::io::AsyncRead + Unpin, W: tokio::io::AsyncWrite + Unpin>(
    reader: &mut BufReader<R>,
    writer: &mut W,
    action: Action,
    options: RequestOptions,
) -> anyhow::Result<()> {
    // Read the connected handshake
    let mut handshake = String::new();
    reader.read_line(&mut handshake).await?;

    // Send the action
    let mut message: serde_json::Value = serde_json::from_str(&action.to_json()?)?;
    if options.dry_run {
        message["dry_run"] = serde_json::json!(true);
    }
    if let Some(timeout_ms) = options.timeout_ms {
        message["timeout_ms"] = serde_json::json!(timeout_ms);
    }
    writer
        .write_all(format!("{}\n", serde_json::to_string(&message)?).as_bytes())
        .await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    // Pretty-print it
    let parsed: serde_json::Value = serde_json::from_str(&response)?;

    // If it's a status command, just print uptime
    if matches!(action, Action::Ping) && parsed["type"] == "pong" {
        println!("deskbrid daemon is running");
        return Ok(());
    }

    // For all other commands, pretty-print the data field
    if let Some(data) = parsed.get("data") {
        println!("{}", serde_json::to_string_pretty(data)?);
    } else if let Some(error) = parsed.get("error") {
        eprintln!(
            "Error: {}",
            error["message"].as_str().unwrap_or("unknown error")
        );
        std::process::exit(1);
    } else {
        println!("{}", serde_json::to_string_pretty(&parsed)?);
    }

    Ok(())
}

/// Send a raw action envelope by name + data JSON and return the parsed response.
///
/// Used by the REPL so it can dispatch any of the 250+ action types without
/// having to mirror the `Action` enum. Returns the daemon's parsed response
/// envelope (caller pretty-prints). TCP/Unix transport and auth are handled
/// the same way as `send_one_shot`.
pub async fn send_raw(
    action_type: &str,
    data: serde_json::Value,
    options: RequestOptions,
) -> anyhow::Result<serde_json::Value> {
    if let (Some(addr), Some(token)) = (tcp_addr(), tcp_token()) {
        return send_raw_tcp(action_type, data, options, &addr, token).await;
    }
    send_raw_unix(action_type, data, options).await
}

async fn send_raw_unix(
    action_type: &str,
    data: serde_json::Value,
    options: RequestOptions,
) -> anyhow::Result<serde_json::Value> {
    let sock = socket_path();
    let stream = UnixStream::connect(&sock).await.context(format!(
        "cannot connect to daemon at {}. Is deskbrid running?",
        sock
    ))?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    run_raw(&mut reader, &mut writer, action_type, data, options).await
}

async fn send_raw_tcp(
    action_type: &str,
    data: serde_json::Value,
    options: RequestOptions,
    addr: &str,
    token: String,
) -> anyhow::Result<serde_json::Value> {
    let stream = tokio::net::TcpStream::connect(addr)
        .await
        .context(format!("cannot connect to daemon at {}", addr))?;
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    let auth = serde_json::json!({"type": "auth", "token": token});
    writer
        .write_all(format!("{}\n", serde_json::to_string(&auth)?).as_bytes())
        .await?;
    let mut auth_response = String::new();
    reader.read_line(&mut auth_response).await?;
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&auth_response)
        && parsed.get("status") == Some(&serde_json::Value::String("error".into()))
    {
        let msg = parsed["error"]["message"]
            .as_str()
            .unwrap_or("authentication failed");
        anyhow::bail!("TCP auth failed: {}", msg);
    }
    run_raw(&mut reader, &mut writer, action_type, data, options).await
}

async fn run_raw<R: tokio::io::AsyncRead + Unpin, W: tokio::io::AsyncWrite + Unpin>(
    reader: &mut BufReader<R>,
    writer: &mut W,
    action_type: &str,
    data: serde_json::Value,
    options: RequestOptions,
) -> anyhow::Result<serde_json::Value> {
    // Read handshake
    let mut handshake = String::new();
    reader.read_line(&mut handshake).await?;

    // Build envelope
    let id = format!("repl-{}", uuid_like());
    let mut envelope = serde_json::json!({
        "type": action_type,
        "id": id,
        "data": data,
    });
    if options.dry_run {
        envelope["dry_run"] = serde_json::json!(true);
    }
    if let Some(timeout_ms) = options.timeout_ms {
        envelope["timeout_ms"] = serde_json::json!(timeout_ms);
    }
    writer
        .write_all(format!("{}\n", serde_json::to_string(&envelope)?).as_bytes())
        .await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;
    let parsed: serde_json::Value = serde_json::from_str(&response)?;
    Ok(parsed)
}

/// Tiny non-cryptographic ID for tracing REPL requests. Doesn't need to be unique
/// across processes — just unique within this REPL session.
fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}
