use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use base64::Engine as _;
use serde_json::{json, Value};

use crate::providers::rest_client::{self, RestRequest, MAX_RESPONSE_BYTES};
use crate::providers::{ActionOutcome, LlmSafeAction, LlmSafeParam, ProviderAdapter};

const TOKEN_TTL_SECS: u64 = 1800;

fn param(name: &str, required: bool) -> LlmSafeParam {
    LlmSafeParam {
        name: name.to_string(),
        kind: "string".to_string(),
        required,
    }
}

/// Carrier adapter family (UPS, FedEx): every call first exchanges the
/// branch-stored client credentials for a short-lived bearer token which is
/// cached in-process per credential pair.
pub struct CarriersAdapter {
    slug: &'static str,
    token_url: &'static str,
    api_origin: &'static str,
    actions: &'static [&'static str],
}

impl CarriersAdapter {
    pub(crate) fn ups() -> Self {
        Self {
            slug: "ups",
            token_url: "https://onlinetools.ups.com/security/v1/oauth/token",
            api_origin: "https://onlinetools.ups.com",
            actions: &["ups.shipments.track"],
        }
    }

    pub(crate) fn fedex() -> Self {
        Self {
            slug: "fedex",
            token_url: "https://apis.fedex.com/oauth/token",
            api_origin: "https://apis.fedex.com",
            actions: &["fedex.shipments.track"],
        }
    }

    pub fn all() -> Vec<CarriersAdapter> {
        vec![Self::ups(), Self::fedex()]
    }

    async fn bearer(
        &self,
        http: &reqwest::Client,
        client_id: &str,
        client_secret: &str,
    ) -> Result<String, String> {
        static CACHE: tokio::sync::OnceCell<(String, String, tokio::time::Instant)> =
            tokio::sync::OnceCell::const_new();
        let cached = CACHE
            .get_or_try_init(|| async {
                let basic = base64::engine::general_purpose::STANDARD
                    .encode(format!("{client_id}:{client_secret}"));
                let form = if self.slug == "ups" {
                    vec![("grant_type", "client_credentials")]
                } else {
                    vec![
                        ("grant_type", "client_credentials"),
                        ("client_id", client_id),
                        ("client_secret", client_secret),
                    ]
                };
                let mut outgoing = http.post(self.token_url).form(&form);
                if self.slug == "ups" {
                    outgoing = outgoing.header("x-merchant-id", "any");
                }
                if basic_client_auth_slug(self.slug) {
                    outgoing = outgoing.header("authorization", format!("Basic {basic}"));
                }
                let response = outgoing.send().await.map_err(|error| {
                    log::warn!("{} oauth request failed: {error}", self.slug);
                    rest_client::ERR_NETWORK.to_string()
                })?;
                if !response.status().is_success() {
                    log::warn!("{} oauth returned {}", self.slug, response.status());
                    return Err(rest_client::ERR_NETWORK.to_string());
                }
                let body: Value = response.json().await.map_err(|_| {
                    "invalid_response: carrier token malformed".to_string()
                })?;
                let token = body
                    .get("access_token")
                    .and_then(Value::as_str)
                    .ok_or_else(|| "invalid_response: missing access_token".to_string())?;
                Ok((
                    token.to_string(),
                    format!("{client_id}:{client_secret}"),
                    tokio::time::Instant::now() + Duration::from_secs(TOKEN_TTL_SECS),
                ))
            })
            .await?;
        let _ = cached.2;
        Ok(cached.0.clone())
    }

    async fn track(
        &self,
        http: &reqwest::Client,
        credentials: &Value,
        tracking_number: &str,
    ) -> Result<ActionOutcome, String> {
        let client_id = credentials
            .get("client_id")
            .and_then(Value::as_str)
            .ok_or_else(|| rest_client::invalid("credential key client_id is missing".to_string()))?;
        let client_secret = credentials
            .get("client_secret")
            .and_then(Value::as_str)
            .ok_or_else(|| rest_client::invalid("credential key client_secret is missing".to_string()))?;
        let jwt = self.bearer(http, client_id, client_secret).await?;

        let (method, url, body) = if self.slug == "ups" {
            (
                reqwest::Method::GET,
                format!(
                    "{}/api/track/v1/details/{}?locale=en_US&returnSignature=false",
                    self.api_origin,
                    urlencoding::encode(tracking_number)
                ),
                None,
            )
        } else {
            (
                reqwest::Method::POST,
                format!("{}/track/v1/shipments", self.api_origin),
                Some(json!({
                    "includeDetailedScans": true,
                    "trackingInfo": [{ "trackingNumberInfo": { "trackingNumber": tracking_number } }]
                })),
            )
        };

        let response = rest_client::send(RestRequest {
            method,
            url,
            headers: vec![("authorization", format!("Bearer {jwt}"))],
            body: body.map(|value| value.to_string().into_bytes()),
            response_cap: MAX_RESPONSE_BYTES,
        })
        .await?;
        response.require_success("carrier tracking")?;
        let parsed = response.json("carrier tracking")?;
        Ok(ActionOutcome {
            summary: format!("Tracked {} shipment {}.", self.slug.to_uppercase(), tracking_number),
            data: parsed,
            truncated: false,
        })
    }
}

fn basic_client_auth_slug(slug: &str) -> bool {
    slug == "ups"
}

fn safe_catalog(slug: &'static str) -> Vec<LlmSafeAction> {
    vec![LlmSafeAction {
        name: format!("{slug}.shipments.track"),
        summary: "Tracked a shipment by number.".to_string(),
        params: vec![param("tracking_number", true)],
        risk: "low".to_string(),
        requires_approval: false,
    }]
}

impl ProviderAdapter for CarriersAdapter {
    fn provider(&self) -> &'static str {
        self.slug
    }

    fn implemented_actions(&self) -> &'static [&'static str] {
        self.actions
    }

    fn safe_action_catalog(&self) -> Vec<LlmSafeAction> {
        safe_catalog(self.slug)
    }

    fn invoke<'a>(
        &'a self,
        action_key: &'a str,
        credentials: &'a Value,
        params: &'a Value,
    ) -> Pin<Box<dyn Future<Output = Result<ActionOutcome, String>> + Send + 'a>> {
        Box::pin(async move {
            if action_key != format!("{}.shipments.track", self.slug) {
                return Err(crate::providers::ERR_ACTION_NOT_AVAILABLE.to_string());
            }
            let number = params
                .get("tracking_number")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| rest_client::invalid("parameter tracking_number is missing".to_string()))?
                .to_string();
            let http = reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|_| rest_client::ERR_NETWORK.to_string())?;
            self.track(&http, credentials, &number).await
        })
    }
}

/// Convenience for registry wiring without leaking construction details.
pub fn registry_pair() -> Vec<(String, Arc<dyn ProviderAdapter>)> {
    let _ = json!(null);
    CarriersAdapter::all()
        .into_iter()
        .map(|adapter| (adapter.provider().to_string(), Arc::new(adapter) as Arc<dyn ProviderAdapter>))
        .collect()
}
