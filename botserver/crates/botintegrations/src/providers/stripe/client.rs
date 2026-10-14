//! Stripe REST request policy for the provider adapter (#950 slice 2).
//!
//! Credentials arrive strictly from the Vault envelope loaded by
//! [`crate::providers::invoke_registered`]; they are never logged and never
//! included in error strings. Every call targets `https://api.stripe.com/v1`
//! with bearer authentication; list requests use query parameters while
//! create requests use form-encoded bodies as the official API requires.

use serde_json::Value;

use crate::providers::rest_client::{self, RestRequest, MAX_RESPONSE_BYTES};

const API_ORIGIN: &str = "https://api.stripe.com/v1";
const USER_AGENT: &str = "generalbots-botintegrations";
const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

/// Maximum accepted secret key length; live keys are far shorter, so
/// anything larger is treated as a misconfigured envelope.
const MAX_KEY_LEN: usize = 512;

/// Parsed Stripe credential envelope from Vault.
#[derive(Debug, Clone)]
pub(crate) struct StripeCreds {
    pub(crate) secret_key: String,
}

impl StripeCreds {
    /// Validates the required `api_key` key before any network activity.
    /// Only secret (`sk_...`) or restricted (`rk_...`) keys may drive these
    /// actions; publishable keys and webhook secrets are rejected up front.
    pub(crate) fn parse(credentials: &Value) -> Result<Self, String> {
        let object = match credentials {
            Value::Object(map) => map,
            _ => {
                return Err(rest_client::invalid(
                    "stored credential envelope must be an object".to_string(),
                ))
            }
        };
        let raw = object
            .get("api_key")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                rest_client::invalid("credential key api_key is missing or empty".to_string())
            })?;
        if raw.len() > MAX_KEY_LEN {
            return Err(rest_client::invalid(format!(
                "credential key api_key must be at most {MAX_KEY_LEN} characters"
            )));
        }
        if !raw.starts_with("sk_") && !raw.starts_with("rk_") {
            return Err(rest_client::invalid(
                "credential key api_key must be a secret (sk_) or restricted (rk_) key".to_string(),
            ));
        }
        Ok(Self {
            secret_key: raw.to_string(),
        })
    }
}

fn authed_headers(creds: &StripeCreds) -> Vec<(&'static str, String)> {
    vec![
        ("authorization", format!("Bearer {}", creds.secret_key)),
        ("user-agent", USER_AGENT.to_string()),
    ]
}

/// Sends an authenticated GET against the Stripe API.
pub(crate) async fn get(
    creds: &StripeCreds,
    path_and_query: &str,
) -> Result<rest_client::RestResponse, String> {
    rest_client::send(RestRequest {
        method: reqwest::Method::GET,
        url: format!("{API_ORIGIN}{path_and_query}"),
        headers: authed_headers(creds),
        body: None,
        response_cap: MAX_RESPONSE_BYTES,
    })
    .await
}

/// Sends an authenticated POST with a form-encoded body built from the
/// supplied ordered field pairs (Stripe's documented request format).
pub(crate) async fn post_form(
    creds: &StripeCreds,
    path_and_query: &str,
    fields: Vec<(String, String)>,
) -> Result<rest_client::RestResponse, String> {
    let mut headers = authed_headers(creds);
    headers.push(("content-type", FORM_CONTENT_TYPE.to_string()));
    let body = fields
        .iter()
        .map(|(name, value)| {
            format!(
                "{}={}",
                urlencoding::encode(name),
                urlencoding::encode(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    rest_client::send(RestRequest {
        method: reqwest::Method::POST,
        url: format!("{API_ORIGIN}{path_and_query}"),
        headers,
        body: Some(body.into_bytes()),
        response_cap: MAX_RESPONSE_BYTES,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_secret_and_restricted_keys_only() {
        let ok = StripeCreds::parse(&json!({ "api_key": " sk_test_abc123 " }))
            .unwrap_or_else(|error| panic!("valid envelope rejected: {error}"));
        assert_eq!(ok.secret_key, "sk_test_abc123");
        assert!(StripeCreds::parse(&json!({ "api_key": "rk_live_xyz" })).is_ok());

        assert!(StripeCreds::parse(&json!({})).is_err());
        assert!(StripeCreds::parse(&json!({ "api_key": "" })).is_err());
        assert!(StripeCreds::parse(&json!({ "api_key": "pk_live_publishable" })).is_err());
        assert!(StripeCreds::parse(&json!({ "api_key": "whsec_webhook" })).is_err());
        assert!(StripeCreds::parse(&json!([1])).is_err());
    }

    #[test]
    fn form_bodies_are_url_encoded_pairs() {
        let fields = [("name".to_string(), "Acme & Co".to_string()),
            ("email".to_string(), "a=b@example.com".to_string())];
        let body = fields
            .iter()
            .map(|(name, value)| {
                format!(
                    "{}={}",
                    urlencoding::encode(name),
                    urlencoding::encode(value)
                )
            })
            .collect::<Vec<_>>()
            .join("&");
        assert_eq!(body, "name=Acme%20%26%20Co&email=a%3Db%40example.com");
    }
}
