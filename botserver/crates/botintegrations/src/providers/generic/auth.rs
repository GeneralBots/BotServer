use super::*;

pub(super) fn basic_header(user: &str, password: &str) -> String {
    let raw = format!("{user}:{password}");
    format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(raw))
}

/// Resolves the base URL for the provider, applying credential-derived
/// origins where the official API requires them.
pub(super) fn resolve_origin(spec: &ProviderSpec, credentials: &Value) -> Result<String, String> {
    match spec.origin {
        Origin::Static(origin) => Ok(origin.to_string()),
        Origin::ZendeskSubdomain => {
            let subdomain = cred_str(credentials, "subdomain")?;
            if !subdomain
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                return Err(invalid(
                    "credential key subdomain contains invalid characters".to_string(),
                ));
            }
            Ok(format!(
                "https://{subdomain}.api.zendesk.com/api/v2"
            ))
        }
        Origin::MailchimpDataCenter => {
            let key = cred_str(credentials, "api_key")?;
            let dc = key.rsplit('-').next().unwrap_or_default();
            if dc.len() > 16 || !dc.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(invalid(
                    "credential key api_key has no usable data center suffix".to_string(),
                ));
            }
            Ok(format!("https://{dc}.api.mailchimp.com/3.0"))
        }
        Origin::FromField { field, pattern } => {
            let value = cred_str(credentials, field)?;
            let origin = pattern.replace("{value}", value);
            if !origin.starts_with("https://") {
                return Err(invalid(
                    "credential key base_url must be an https URL".to_string(),
                ));
            }
            Ok(origin)
        }
    }
}

/// Header and query pairs a provider's auth style contributes to a request.
pub(super) type AuthHeaders = (Vec<(&'static str, String)>, Vec<(String, String)>);

pub(super) fn auth_headers_and_query(
    spec: &ProviderSpec,
    credentials: &Value,
) -> Result<AuthHeaders, String> {
    let mut headers = Vec::new();
    let mut query = Vec::new();
    match &spec.auth {
        AuthStyle::Bearer { token_field } => {
            let token = cred_str(credentials, token_field)?;
            headers.push(("authorization", format!("Bearer {token}")));
        }
        AuthStyle::ApiKeyHeader { header, field } => {
            let key = cred_str(credentials, field)?;
            headers.push((*header, key.to_string()));
        }
        AuthStyle::BasicTemplate {
            user_template,
            password_field,
        } => {
            let password = cred_str(credentials, password_field)?;
            let mut user = user_template.to_string();
            while let Some(start) = user.find('{') {
                let Some(end) = user[start..].find('}') else {
                    return Err(invalid("malformed credential template".to_string()));
                };
                let field = &user[start + 1..start + end];
                let value = cred_str(credentials, field)?;
                user.replace_range(start..start + end + 1, value);
            }
            headers.push(("authorization", basic_header(&user, password)));
        }
        AuthStyle::BasicJoin {
            first_field,
            separator,
            second_field,
        } => {
            let first = cred_str(credentials, first_field)?;
            let second = second_field
                .map(|field| cred_str(credentials, field))
                .transpose()?
                .unwrap_or_default();
            if second.contains(*separator) {
                return Err(invalid("credential field contains the separator".to_string()));
            }
            headers.push((
                "authorization",
                basic_header(first, second),
            ));
        }
        AuthStyle::QueryPairs { pairs } => {
            for (name, field) in *pairs {
                let value = cred_str(credentials, field)?;
                query.push(((*name).to_string(), value.to_string()));
            }
        }
        AuthStyle::BodyField { .. } => {
            // Credential material is injected into the JSON body by
            // `flat_body` at invocation time; nothing enters headers.
        }
        AuthStyle::ApiKeyHeaders { pairs } => {
            for (header, field) in *pairs {
                let value = cred_str(credentials, field)?;
                headers.push((*header, value.to_string()));
            }
        }
        AuthStyle::BearerHeaders {
            token_field,
            headers: extra,
        } => {
            let token = cred_str(credentials, token_field)?;
            headers.push(("authorization", format!("Bearer {token}")));
            for (name, value) in *extra {
                headers.push((*name, (*value).to_string()));
            }
        }
    }
    headers.push(("user-agent", "generalbots-botintegrations".to_string()));
    Ok((headers, query))
}

pub(crate) fn require_param_str(params: &Value, name: &str) -> Result<String, String> {
    let text = match params.get(name) {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => {
            return Err(invalid(format!("parameter {name} is missing")));
        }
    };
    if text.is_empty() {
        return Err(invalid(format!("parameter {name} is empty")));
    }
    Ok(text)
}
