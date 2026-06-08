//! MCP parameter types for file and terminal operations.

use serde::Deserialize;

// ── Files ──────────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FilePath {
    #[schemars(description = "File or directory path")]
    pub path: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileRead {
    #[schemars(description = "File path")]
    pub path: String,
    #[schemars(description = "Byte offset to start reading")]
    pub offset: Option<u64>,
    #[schemars(description = "Maximum bytes to read")]
    pub limit: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileWrite {
    #[schemars(description = "File path")]
    pub path: String,
    #[schemars(description = "Content to write")]
    pub content: String,
    #[schemars(description = "Append instead of overwrite")]
    #[serde(default)]
    pub append: bool,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileSearch {
    #[schemars(description = "Search pattern (glob or regex)")]
    pub pattern: String,
    #[schemars(description = "Root directory to search (default: home)")]
    pub root: Option<String>,
    #[schemars(description = "Maximum results")]
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileCopy {
    #[schemars(description = "Source path")]
    pub source: String,
    #[schemars(description = "Destination path")]
    pub destination: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct FileWatch {
    #[schemars(description = "Directory or file path to watch")]
    pub path: String,
    #[schemars(description = "Watch recursively")]
    #[serde(default)]
    pub recursive: bool,
    #[schemars(description = "File patterns to watch (e.g. ['*.rs'])")]
    pub patterns: Option<Vec<String>>,
}

fn default_max_results() -> u32 {
    50
}

// ── Terminal ───────────────────────────────────────────────

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TerminalCreate {
    #[schemars(description = "Shell to use (default: /bin/bash)")]
    pub shell: Option<String>,
    #[schemars(description = "Working directory")]
    pub cwd: Option<String>,
    #[schemars(description = "Terminal rows")]
    pub rows: Option<u16>,
    #[schemars(description = "Terminal columns")]
    pub cols: Option<u16>,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TerminalWrite {
    #[schemars(description = "Terminal ID")]
    pub terminal_id: String,
    #[schemars(description = "Input to send (supports ANSI escape sequences)")]
    pub input: String,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TerminalRead {
    #[schemars(description = "Terminal ID")]
    pub terminal_id: String,
    #[schemars(description = "Maximum bytes to read")]
    pub max_bytes: Option<u64>,
    #[schemars(description = "Flush output buffer before reading")]
    #[serde(default)]
    pub flush: bool,
}

#[derive(Deserialize, schemars::JsonSchema, Default)]
pub struct TerminalResize {
    #[schemars(description = "Terminal ID")]
    pub terminal_id: String,
    #[schemars(description = "Rows")]
    pub rows: u16,
    #[schemars(description = "Columns")]
    pub cols: u16,
}
