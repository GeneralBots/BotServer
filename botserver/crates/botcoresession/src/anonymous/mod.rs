//! Anonymous session support.
//!
//! Split from the former monolithic `anonymous.rs`. The public surface is
//! unchanged: the explicit `pub use anonymous::{...}` list in `lib.rs` still
//! resolves every item.
pub mod cleanup;
pub mod manager;
pub mod types;

pub use cleanup::*;
pub use manager::*;
pub use types::*;
