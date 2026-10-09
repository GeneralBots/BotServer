//! MCP tool surface for botvideo.
//!
//! Split from the former monolithic `mcp_tools.rs`. The public surface is
//! unchanged: callers keep using `botvideo::mcp_tools::{...}`.

pub mod definitions;
pub mod tools;
pub mod types;

pub use definitions::*;
pub use tools::*;
pub use types::*;
