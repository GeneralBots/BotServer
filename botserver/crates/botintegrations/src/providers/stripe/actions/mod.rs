//! Stripe action implementations for the provider adapter (#950 slice 2).
//!
//! Every handler validates its parameters completely before any network
//! activity, requests a single bounded first page, and projects the response
//! down to a small, redaction-safe subset of fields.

pub(crate) use serde_json::{json, Value};
use super::client::{self, StripeCreds};
pub(crate) use crate::providers::rest_client::{ bounded_limit, invalid, optional_text, outcome, push_query_pair, };
pub(crate) use crate::providers::ActionOutcome;

const MAX_EMAIL_LEN: usize = 254;
const MAX_NAME_LEN: usize = 256;
const LIST_LIMIT_MAX: usize = 50;

mod validate;
mod fields;
mod project;
mod email;
mod tests;

use validate::*;
use fields::*;
use project::*;
pub(crate) use email::*;
