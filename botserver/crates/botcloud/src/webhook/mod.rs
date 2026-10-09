//! Stripe Webhook — processes asynchronous payment events.
//!
//! Handled events:
//! - `checkout.session.completed` → paid invoice, CRM deal won, GL entry, subscription
//! - `invoice.paid` → recurring charge paid
//! - `invoice.payment_failed` → payment failure
//! - `customer.subscription.*` → subscription lifecycle

pub(crate) use axum::{ extract::State, http::{HeaderMap, StatusCode}, routing::post, Json, Router, };
pub(crate) use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::{integration, notifier, SaasService};

mod part1;
mod part2;

pub use part1::*;
use part2::*;
