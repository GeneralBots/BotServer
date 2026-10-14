use axum::{extract::{Path, Query, State}, http::{HeaderMap, StatusCode}, middleware, routing::{get, post, put, delete}, Json, Router};
use axum::response::{IntoResponse, Response};
use axum::body::Body;
use diesel::deserialize::QueryableByName;
use diesel::prelude::*;
use diesel::{ExpressionMethods, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(QueryableByName, Debug)]
struct OrgRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}

#[derive(QueryableByName, Debug)]
struct BranchIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
}

use crate::{integration, notifier, CalculatorPayload, SaasConfig, SaasService};


/// JWT authentication middleware for cloud API routes.
/// Validates Bearer token on all routes except `/api/cloud/auth/*`.
async fn cloud_jwt_middleware(
    axum::Extension(jwt_secret): axum::Extension<String>,
    request: axum::http::Request<Body>,
    next: middleware::Next,
) -> Response {
    let path = request.uri().path().to_string();

    // Skip auth for public endpoints
    if path.starts_with("/api/cloud/auth/")
        || path.starts_with("/api/domains/resolve")
        || path.starts_with("/api/domains/tls-ask")
        || path == "/api/cloud/tenant/settings/oauth/google/callback"
    {
        return next.run(request).await;
    }

    // Public GET endpoints for anonymous product/plan browsing
    if request.method() == "GET"
        && (path.starts_with("/api/cloud/store")
            || path.starts_with("/api/cloud/plans")
            || path.starts_with("/api/cloud/offers")
            || path.starts_with("/api/cloud/llm-providers")
            || path.starts_with("/api/products/")
            || path.starts_with("/api/catalog/"))
        {
            return next.run(request).await;
        }

    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    match auth_header {
        Some(token) => {
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() != 3 {
                let err = serde_json::json!({"error": "Invalid token format"});
                return (StatusCode::UNAUTHORIZED, Json(err)).into_response();
            }
            let (header_b64, _payload_b64, sig_b64) = (parts[0], parts[1], parts[2]);
            let message = format!("{}.{}", header_b64, parts[1]);
            let expected_sig = jwt_sign_inner(&message, jwt_secret.as_bytes());
            if sig_b64 != expected_sig {
                let err = serde_json::json!({"error": "Invalid token signature"});
                return (StatusCode::UNAUTHORIZED, Json(err)).into_response();
            }
            next.run(request).await
        }
        None => {
            let err = serde_json::json!({"error": "Missing Authorization header"});
            (StatusCode::UNAUTHORIZED, Json(err)).into_response()
        }
    }
}

/// HMAC-SHA256 sign a message and return the base64url-encoded signature.

// #1370 — this module used to be a single 3646-line file. It is split by
// responsibility: api_types (JWT + payloads), helpers (validation and user
// provisioning), signup, login/login_helpers, jwt_api, orgs, workspaces_api,
// resources, branches, catalog, billing_api, llm_api, byok. Handlers were
// private before the split, so they are re-exported at crate visibility.
mod api_tests;
mod api_types;
mod billing_api;
mod branches;
mod byok;
mod catalog;
mod checkout;
mod helpers;
mod jwt_api;
mod llm_api;
mod login;
mod login_helpers;
mod orgs;
mod resources;
mod signup;
mod workspaces_api;

pub use api_types::*;
pub use billing_api::*;
pub use branches::*;
pub use jwt_api::*;
pub use workspaces_api::*;
pub(crate) use byok::*;
pub(crate) use catalog::*;
pub(crate) use checkout::*;
pub(crate) use helpers::*;
pub(crate) use llm_api::*;
pub(crate) use login::*;
pub(crate) use login_helpers::*;
pub(crate) use orgs::*;
pub(crate) use resources::*;
pub(crate) use signup::*;
pub fn configure_cloud_api_routes(config: SaasConfig) -> Router<Arc<SaasService>> {
    let jwt_secret = config.jwt_secret.clone();
    Router::new()
        // Auth
        .route("/api/cloud/auth/login", post(handle_login))
        .route("/api/cloud/auth/signup", post(handle_signup))
        // Checkout / Plans
        .route("/api/cloud/checkout", post(handle_checkout))
        .route("/api/cloud/checkout/success", get(checkout_success))
        .route("/api/cloud/plans", get(list_plans))
        .route("/api/cloud/plans/:plan_id", get(get_plan_detail))
        // Organizations
        .route("/api/cloud/organizations", get(list_organizations).post(create_organization))
        .route("/api/cloud/organizations/:org_id", get(get_organization).put(update_organization).delete(delete_organization))
        .route("/api/cloud/organizations/:org_id/billing", get(org_billing_portal))
        // Branches per organization
        .route("/api/cloud/organizations/:org_id/branches", get(list_branches).post(create_branch_handler))
        .route("/api/cloud/organizations/:org_id/branches/:branch_id", put(update_branch_handler).delete(delete_branch_handler))
        // Workspaces per organization
        .route("/api/cloud/organizations/:org_id/workspaces", get(list_workspaces).post(create_workspace))
        .route("/api/cloud/organizations/:org_id/workspaces/:ws_id", put(update_workspace).delete(delete_workspace))
        // Resources per workspace
        .route("/api/cloud/organizations/:org_id/workspaces/:ws_id/resources", get(list_workspace_resources).post(assign_resource))
        .route("/api/cloud/organizations/:org_id/workspaces/:ws_id/resources/:res_id", delete(remove_resource))
        // Services (purchased add-ons)
        .route("/api/cloud/bots", get(list_bots))
        .route("/api/cloud/services", get(list_services))
        .route("/api/cloud/services/:id/cancel", post(cancel_service))
        // Invoices
        .route("/api/cloud/invoices", get(list_invoices))
        // Vouchers
        .route("/api/cloud/vouchers", post(crate::vouchers::create_voucher).get(crate::vouchers::list_vouchers))
        .route("/api/cloud/vouchers/redeem", post(crate::vouchers::redeem_voucher))
        .route("/api/cloud/vouchers/my", get(crate::vouchers::get_my_redemptions))
        // Payment cards (Stripe SetupIntent / hosted Checkout setup mode)
        .route("/api/cloud/payment-cards", get(crate::payment_cards::list_payment_cards))
        .route("/api/cloud/payment-cards/setup", post(crate::payment_cards::create_payment_card_setup))
        .route("/api/cloud/payment-cards/setup-intent", post(crate::payment_cards::create_payment_card_setup_intent))
        .route("/api/cloud/payment-cards/:pm_id/default", post(crate::payment_cards::set_default_payment_card))
        .route("/api/cloud/payment-cards/:pm_id", delete(crate::payment_cards::delete_payment_card))
        // Store items
        .route("/api/cloud/store", get(list_store_items))
        .route("/api/cloud/store/purchase", post(handle_store_purchase))
        .route("/api/cloud/billing-portal", get(billing_portal))
        // Profile
        .route("/api/cloud/profile", get(get_profile).post(update_profile).put(update_profile))
        // Top-up (Special Offers)
        .route("/api/cloud/topup", post(handle_topup))
        // App Store Publishing Consultancy
        .route("/api/cloud/appstore/purchase", post(handle_appstore_purchase))
        // Offers (combo bundles)
        .route("/api/cloud/offers", get(list_offers))
        // LLM Providers catalog
        .route("/api/cloud/llm-providers", get(list_llm_providers))
        // BYOK (Bring Your Own Key) — encrypted server-side storage
        .route("/api/cloud/tenant/settings/byok", post(handle_save_byok))
        .route("/api/cloud/tenant/settings/oauth/:provider/start", get(handle_oauth_start))
        .route("/api/cloud/tenant/settings/oauth/google/callback", get(handle_google_oauth_callback))
        // Admin
        .route("/api/cloud/admin/server-capacity", get(get_server_capacity))
        // Domains (CRUD — admin only, all require JWT)
        .route("/api/cloud/domains", get(crate::domains::list_domains).post(crate::domains::create_domain))
        .route("/api/cloud/domains/:id", put(crate::domains::update_domain).delete(crate::domains::delete_domain))
        // Domain resolution (public — no JWT required)
        .route("/api/domains/resolve", get(crate::domains::resolve_domain))
        // Caddy on-demand TLS decision endpoint (public — the proxy itself calls it)
        .route("/api/domains/tls-ask", get(crate::domains::tls_ask_domain))
        // JWT auth middleware — protects all routes except /api/cloud/auth/*
        .layer(middleware::from_fn(cloud_jwt_middleware))
        .layer(axum::Extension(jwt_secret))
}

