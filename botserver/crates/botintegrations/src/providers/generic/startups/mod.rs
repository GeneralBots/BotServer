//! Generic provider specs for the startup toolset, grouped per vendor.

pub(crate) use super::helpers::{json, json_req, resource_id, s, s_req};
pub(crate) use super::{ActionSpec, AuthStyle, Origin, ParamKind, ParamSpec, ProviderSpec, Risk};

mod apollo;
mod canny;
mod streak;
mod zendesk;

pub use apollo::*;
pub use canny::*;
pub use streak::*;
pub use zendesk::*;
