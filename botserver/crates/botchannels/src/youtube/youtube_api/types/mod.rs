//! YouTube Data API Request/Response Types
//!
//! Contains all public types for making requests and handling responses
//! from the YouTube Data API v3.


// ============================================================================
// Request Types
// ============================================================================

pub(crate) use serde::{Deserialize, Serialize};

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
