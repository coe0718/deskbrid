use super::action::Action;
use super::types::RequestOptions;

impl Action {
    /// Parse an incoming NDJSON line into an Action.
    pub fn from_json(line: &str) -> anyhow::Result<(String, Action)> {
        super::parse::from_json(line)
    }

    /// Parse an incoming NDJSON line into an Action plus request-level options.
    pub fn from_json_with_options(line: &str) -> anyhow::Result<(String, Action, RequestOptions)> {
        super::parse::from_json_with_options(line)
    }
}
