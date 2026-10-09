//! Message template keywords: send, create, fetch and variable extraction.

pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use log::{debug, info, trace};
pub(crate) use rhai::{Array, Dynamic, Engine, Map};
pub(crate) use std::sync::Arc;

mod registration;
mod sending;
mod templates;
mod tests;

pub use registration::*;
use sending::*;
use templates::*;
