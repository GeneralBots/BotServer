//! Stripe payment card management (SetupIntent flow).
//!
//! Cards are collected exclusively on Stripe's side — either via a hosted
//! Checkout session in `setup` mode (used by the cloud UI) or via a raw
//! SetupIntent `client_secret` for clients embedding Stripe Elements. Card
//! numbers never transit our servers or database (PCI SAQ-A); we persist only
//! display metadata (brand, last4, expiry) synced from Stripe webhooks.
//!
//! RBAC: cards are scoped to the authenticated user's branch. The branch owner
//! (or the SaaS super-admin) is the only actor allowed to add / set default /
//! delete cards; every mutation is recorded in `cloud_audit_log`.

pub(crate) use axum::{ extract::{Path, State}, http::{HeaderMap, StatusCode}, Json, };
pub(crate) use diesel::prelude::*;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::api::{base64_url_decode, get_branch_id_from_jwt, is_super_admin};
pub(crate) use crate::SaasService;

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
