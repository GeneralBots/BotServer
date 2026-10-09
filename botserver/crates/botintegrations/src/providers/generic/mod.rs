//! Declarative REST provider engine (#939 wave: token/key/basic providers).
//!
//! Each provider is a static [`ProviderSpec`]: an API origin, a credential
//! style and a table of [`ActionSpec`] entries (method, path template,
//! parameters, risk). The engine turns a Vault envelope plus validated
//! parameters into live calls through the shared `rest_client`, so a new
//! provider costs only data - no bespoke request plumbing.
//!
//! Security contract (same as the bespoke adapters):
//! - credentials load strictly from Vault immediately before invocation;
//! - every outcome passes through [`crate::providers::redact_credentials`];
//! - error strings are static sentinels; provider details stay in logs.

pub(crate) use std::future::Future;
pub(crate) use std::pin::Pin;
pub(crate) use serde_json::Value;
pub(crate) use crate::providers::rest_client::{self, RestRequest, MAX_RESPONSE_BYTES};
pub(crate) use crate::providers::{ActionOutcome, LlmSafeAction, LlmSafeParam, ProviderAdapter};

pub mod commerce;
pub mod community;
pub mod crm_ops;
pub mod developer;
pub mod devops;
pub mod devplatform;
pub mod finplatform;
pub mod finance;
pub mod helpers;
pub mod lifestyle;
pub mod observability;
pub mod payload;

use payload::{build_body, build_url_from_parts, flat_body, shape_outcome};
pub mod payments;
pub mod productivity;
pub mod recruiting;
pub mod small_business;
pub mod social_messaging;
pub mod social_platforms;
pub mod startups;
pub mod workdocs;

use base64::Engine as _;

/// Maximum number of list items surfaced from one outcome; larger arrays
/// are cut and flagged so chat payloads stay bounded.
pub(crate) const MAX_LIST_ITEMS: usize = 25;

/// Risk classification mirroring the integration catalog profiles.

#[cfg(test)]
mod tests;

mod spec;
mod auth;
mod adapter;

pub use spec::*;
pub(crate) use auth::*;
pub use adapter::*;
