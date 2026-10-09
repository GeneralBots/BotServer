//! WeChat type definitions for API requests and responses


// ============================================================================
// Token Types
// ============================================================================

pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
