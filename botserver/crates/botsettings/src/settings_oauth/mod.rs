//! OAuth account linking for the Settings page (#899).
//!
//! Implements the authorization-code + PKCE flow for Google, Microsoft and
//! GitHub and binds the resulting external identity to the authenticated
//! local user. Provider configuration is read from the bot config in Vault
//! (same keys the login flow uses); linked identities are stored in
//! `oauth_account_links` and the access/refresh tokens are persisted in
//! Vault under `secret/gbo/oauth/{user_id}/{provider}` — never in the DB or
//! logs.

pub(crate) use axum::{ extract::{Path, Query, State}, http::StatusCode, response::{Html, IntoResponse, Redirect, Response}, Json, };
pub(crate) use diesel::RunQueryDsl;
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use botcore::shared::state::AppState;
pub(crate) use botcoreoauth::{OAuthConfig, OAuthProvider, OAuthState};
pub(crate) use crate::audit_log::record_audit_event;
pub(crate) use crate::settings_api::{get_conn, resolve_user_id};

/// Providers the Settings page offers for account linking.
const LINKABLE_PROVIDERS: &[&str] = &["google", "microsoft", "github"];

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linkable_providers() {
        assert!(LINKABLE_PROVIDERS.contains(&"google"));
        assert!(LINKABLE_PROVIDERS.contains(&"microsoft"));
        assert!(LINKABLE_PROVIDERS.contains(&"github"));
        assert_eq!(LINKABLE_PROVIDERS.len(), 3);
    }

    #[test]
    fn test_provider_parse() {
        assert_eq!(OAuthProvider::parse("google"), Some(OAuthProvider::Google));
        assert_eq!(
            OAuthProvider::parse("microsoft"),
            Some(OAuthProvider::Microsoft)
        );
        assert_eq!(OAuthProvider::parse("github"), Some(OAuthProvider::GitHub));
        assert_eq!(OAuthProvider::parse("unknown"), None);
    }
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
use part3::*;
