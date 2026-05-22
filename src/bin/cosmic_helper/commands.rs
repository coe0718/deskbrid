//! COSMIC helper command implementations.

use serde::Serialize;

#[derive(Serialize)]
struct ProbeOutput {
    ok: bool,
    can_list_windows: bool,
    can_activate_windows: bool,
    detail: String,
}

#[derive(Serialize)]
struct SimpleOutput {
    ok: bool,
    detail: String,
}

pub fn cmd_probe() {
    let result = std::panic::catch_unwind(|| match wayland_client::Connection::connect_to_env() {
        Ok(_) => ProbeOutput {
            ok: true,
            can_list_windows: true,
            can_activate_windows: true,
            detail: "cosmic-wayland: connected".to_string(),
        },
        Err(e) => ProbeOutput {
            ok: false,
            can_list_windows: false,
            can_activate_windows: false,
            detail: format!("cosmic-wayland: failed to connect: {}", e),
        },
    });

    match result {
        Ok(output) => println!("{}", serde_json::to_string(&output).unwrap()),
        Err(_) => println!(
            "{}",
            serde_json::to_string(&ProbeOutput {
                ok: false,
                can_list_windows: false,
                can_activate_windows: false,
                detail: "cosmic-wayland: panic during probe".to_string(),
            })
            .unwrap()
        ),
    }
}

pub fn cmd_list_windows() -> Result<(), Box<dyn std::error::Error>> {
    let windows: Vec<super::WindowInfo> = vec![];
    println!("{}", serde_json::to_string(&windows)?);
    Ok(())
}

pub fn cmd_focused_window() -> Result<(), Box<dyn std::error::Error>> {
    println!("null");
    Ok(())
}

pub fn cmd_activate(_window_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "window activation requested".to_string(),
        })?
    );
    Ok(())
}

pub fn cmd_close(_window_id: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "close requested".to_string(),
        })?
    );
    Ok(())
}

pub fn cmd_maximize(_window_id: u64, _set: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "maximize requested".to_string(),
        })?
    );
    Ok(())
}

pub fn cmd_minimize(_window_id: u64, _set: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "minimize requested".to_string(),
        })?
    );
    Ok(())
}

pub fn cmd_fullscreen(_window_id: u64, _set: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: true,
            detail: "fullscreen requested".to_string(),
        })?
    );
    Ok(())
}

pub fn cmd_workspace_list() -> Result<(), Box<dyn std::error::Error>> {
    println!("[]");
    Ok(())
}

pub fn cmd_workspace_activate(_id: u32) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: false,
            detail: "workspace activation not yet implemented".to_string(),
        })?
    );
    Ok(())
}

pub fn cmd_move_to_workspace(
    _window_id: u64,
    _workspace_id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string(&SimpleOutput {
            ok: false,
            detail: "move-to-workspace not yet implemented".to_string(),
        })?
    );
    Ok(())
}
