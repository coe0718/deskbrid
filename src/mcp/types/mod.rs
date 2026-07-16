//! Input parameter types for MCP tools, with JsonSchema derives for tool discovery.
//! Each submodule groups types by domain; all types are re-exported here for
//! backward compatibility with `use crate::mcp::types::*`.

mod capture;
mod display;
mod files;
mod input;
mod system;

mod vision;

pub use capture::*;
pub use display::*;
pub use files::*;
pub use input::*;
pub use system::*;
pub use vision::*;
