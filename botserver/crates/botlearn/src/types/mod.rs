//! Type surface for botlearn.
//!
//! Split from the former monolithic `types.rs`. All items remain reachable at
//! `botlearn::types::*`, so the explicit `pub use types::{...}` list in
//! `lib.rs` and the `creator` module are unaffected.
pub mod course;
pub mod lesson;
pub mod progress;
pub mod quiz;
pub mod statistics;

pub use course::*;
pub use lesson::*;
pub use progress::*;
pub use quiz::*;
pub use statistics::*;
