// Canonical definition lives in `crate::core` (#1497), which is always
// available; this feature-gated module re-exports it so
// `botschema::tasks::tasks` keeps resolving.
pub use crate::core::tasks;

pub use self::tasks::*;
