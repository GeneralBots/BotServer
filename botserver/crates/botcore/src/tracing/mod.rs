//! W3C Distributed Tracing module
//!
//! Implements the W3C Trace Context specification for distributed tracing
//! across service boundaries. Provides traceparent header generation/parsing,
//! Tower Layer/Service middleware for server-side tracing, and reqwest layer
//! for client-side propagation.
//!
//! No unwrap() or expect() are used in this module.

//! W3C Trace Context propagation: spans, Tower/reqwest layers and subscriber setup.

pub(crate) use std::time::{Duration, SystemTime};
pub(crate) use uuid::Uuid;

const TRACEPARENT_VERSION: &str = "00";
const TRACE_FLAGS_SAMPLED: u8 = 0x01;

mod span;
mod middleware;
mod reqwest_layer;
mod subscriber;
#[cfg(test)]
mod tests;

pub use span::*;
pub use middleware::*;
pub use reqwest_layer::*;
pub use subscriber::*;
