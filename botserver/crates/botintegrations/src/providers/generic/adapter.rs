use super::*;

/// Builds the flat credential-signed body used by `AuthStyle::BodyField`
/// providers: every declared parameter present in `params` becomes a
pub struct GenericAdapter {
    spec: &'static ProviderSpec,
}

impl GenericAdapter {
    pub const fn new(spec: &'static ProviderSpec) -> Self {
        Self { spec }
    }

    pub(super) fn find_action(&self, action: &str) -> Option<&'static ActionSpec> {
        self.spec.actions.iter().find(|entry| entry.key == action)
    }
}

impl ProviderAdapter for GenericAdapter {
    fn provider(&self) -> &'static str {
        self.spec.slug
    }

    fn implemented_actions(&self) -> &'static [&'static str] {
        self.spec.action_keys
    }

    fn safe_action_catalog(&self) -> Vec<LlmSafeAction> {
        self.spec
            .actions
            .iter()
            .map(|action| LlmSafeAction {
                name: action.key.to_string(),
                summary: action.summary.to_string(),
                params: action
                    .params
                    .iter()
                    .map(|param| LlmSafeParam {
                        name: param.name.to_string(),
                        kind: param.kind.as_str().to_string(),
                        required: param.required,
                    })
                    .collect(),
                risk: match action.risk {
                    Risk::Low => "low",
                    Risk::Medium => "medium",
                    Risk::High => "high",
                }
                .to_string(),
                requires_approval: !action.is_read(),
            })
            .collect()
    }

    fn invoke<'a>(
        &'a self,
        action: &'a str,
        credentials: &'a Value,
        params: &'a Value,
    ) -> Pin<Box<dyn Future<Output = Result<ActionOutcome, String>> + Send + 'a>> {
        Box::pin(async move {
            let Some(spec) = self.find_action(action) else {
                return Err(crate::providers::ERR_ACTION_NOT_AVAILABLE.to_string());
            };
            let origin = resolve_origin(self.spec, credentials)?;
            let (mut headers, auth_query) = auth_headers_and_query(self.spec, credentials)?;

            let mut query = auth_query;
            for (name, field) in spec.query {
                if let Some(value) = params.get(*field) {
                    if let Some(text) = value.as_str() {
                        if !text.trim().is_empty() {
                            query.push(((*name).to_string(), text.trim().to_string()));
                        }
                    }
                }
            }
            let url = build_url_from_parts(&origin, spec, params, query)?;
            let credential_body = match &self.spec.auth {
                AuthStyle::BodyField { field } => {
                    Some(flat_body(credentials, field, spec, params)?)
                }
                _ => None,
            };
            if credential_body.is_some() || (spec.method != "GET" && build_body(spec, params)?.is_some()) {
                let body = match credential_body {
                    Some(body) => body,
                    None => build_body(spec, params)?.unwrap_or_default(),
                };
                headers.push(("content-type", "application/json".to_string()));
                let response = rest_client::send(RestRequest {
                    method: reqwest::Method::from_bytes(spec.method.as_bytes())
                        .map_err(|_| invalid("unsupported method".to_string()))?,
                    url,
                    headers,
                    body: Some(body),
                    response_cap: MAX_RESPONSE_BYTES,
                })
                .await?;
                response.require_success(spec.summary)?;
                return Ok(shape_outcome(spec, response.status, &response.body));
            }
            let response = rest_client::send(RestRequest {
                method: reqwest::Method::from_bytes(spec.method.as_bytes())
                    .map_err(|_| invalid("unsupported method".to_string()))?,
                url,
                headers,
                body: None,
                response_cap: MAX_RESPONSE_BYTES,
            })
            .await?;
            response.require_success(spec.summary)?;
            Ok(shape_outcome(spec, response.status, &response.body))
        })
    }
}
