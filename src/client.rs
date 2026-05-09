use crate::protocol::Action;
use anyhow::Context;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const SOCKET_PATH: &str = "/run/user/1000/deskbrid.sock";

/// Connect to the daemon, send a one-shot action, and print the response.
pub async fn send_one_shot(action: Action) -> anyhow::Result<()> {
    let stream = UnixStream::connect(SOCKET_PATH)
        .await
        .context(format!("cannot connect to daemon at {}. Is deskbrid running?", SOCKET_PATH))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read the connected handshake
    let mut handshake = String::new();
    reader.read_line(&mut handshake).await?;
    // Skip it — we don't need the connected message

    // Send the action
    let json = action.to_json()? + "\n";
    writer.write_all(json.as_bytes()).await?;

    // Read response
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    // Pretty-print it
    let parsed: serde_json::Value = serde_json::from_str(&response)?;

    // If it's a status command, just print uptime
    if matches!(action, Action::Ping) {
        if parsed["type"] == "pong" {
            println!("deskbrid daemon is running");
            return Ok(());
        }
    }

    // For all other commands, pretty-print the data field
    if let Some(data) = parsed.get("data") {
        println!("{}", serde_json::to_string_pretty(data)?);
    } else if let Some(error) = parsed.get("error") {
        eprintln!("Error: {}", error["message"].as_str().unwrap_or("unknown error"));
        std::process::exit(1);
    } else {
        println!("{}", serde_json::to_string_pretty(&parsed)?);
    }

    Ok(())
}
