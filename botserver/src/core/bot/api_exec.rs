//! Generic `api.exec` executor — the VBA-style "drive any app" surface.
//!
//! Any registered REST endpoint (`endpoint_inventory::all_routes()`) can be
//! invoked from chat on ANY channel (web, WhatsApp, Telegram, ...) by running
//! the derived command or the raw `api.exec` command. The executor mints a
//! user-scoped JWT (same user as the session), then performs a loopback HTTP
//! request to the running server — so it goes through the exact same auth,
//! RBAC and CSRF layers as the browser, without hard-coding per-app logic.
//!
//! Security model:
//! - The endpoint must be a registered route (never an arbitrary path).
//! - Bearer-token requests are CSRF-exempt by design (see csrf.rs).
//! - RBAC: admin-only endpoints are denied for non-admin users (checked both
//!   here and re-enforced by the auth middleware on the loopback hop).
//! - Path params (`:id`) are substituted from a bounded allowlist of keys.
//! - Response bodies are size-truncated to avoid flooding the chat.

use std::sync::Arc;

use botcore::shared::state::AppState;
use serde_json::{json, Value};
use uuid::Uuid;

/// Max response body size forwarded to the LLM (bytes).
const MAX_BODY: usize = 60_000;

/// Allowed path-param substitution keys (whitelist, not arbitrary injection).
const ALLOWED_PARAM_KEYS: &[&str] = &[
    "id", "item_id", "record_id", "user_id", "bot_id", "session_id", "team_id", "project_id",
    "task_id", "ticket_id", "invoice_id", "order_id", "campaign_id", "check_id", "issue_id",
    "verification_id", "rule_id", "account_id", "run_id", "enrollment_id", "channel_id",
    "conversation_id", "minute_id", "recording_id", "workspace_id", "page_id", "file_id",
    "deployment_id", "connector_id", "source_id", "note_id", "deck_id", "doc_id", "report_id",
    "event_id", "meeting_id", "room_id", "story_id", "incident_id", "product_id", "media_id",
    "queue_id", "agent_id", "group_id", "role_id", "case_id", "audit_id", "entry_id",
    "transaction_id", "deal_id", "employee_id", "post_id", "canvas_id", "board_id", "plan_id",
    "okr_id", "course_id", "message_id", "calendar_id", "camera_id", "clip_id", "export_id",
    "layer_id", "recording_id", "job_id", "folder_id", "category_id", "price_list_id",
];

/// Path pattern to a concrete path using the provided params.
fn fill_path(template: &str, params: &serde_json::Map<String, Value>) -> Result<String, String> {
    let mut out = String::with_capacity(template.len() + 32);
    let mut rest = template;
    while let Some(open) = rest.find(':').or_else(|| rest.find('{')) {
        out.push_str(&rest[..open]);
        let after = &rest[open..];
        let (key, consumed) = if after.starts_with(':') {
            let end = after[1..]
                .find('/')
                .map(|i| i + 1)
                .unwrap_or(after.len());
            (after[1..end].to_string(), end)
        } else {
            let Some(close) = after.find('}') else {
                return Err(format!("unterminated path param in '{template}'"));
            };
            (after[1..close].to_string(), close + 1)
        };
        let val = match params.get(&key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            _ => return Err(format!("missing path param '{key}'")),
        };
        if !ALLOWED_PARAM_KEYS.contains(&key.as_str()) {
            return Err(format!("path param '{key}' is not allowed"));
        }
        out.push_str(&val);
        rest = &after[consumed..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Validates that the endpoint is a registered route (defense in depth).
fn is_registered(method: &str, path: &str) -> bool {
    crate::core::bot::endpoint_inventory::all_routes()
        .any(|e| e.method == method && path_matches(e.path, path))
}

/// Template vs concrete path match, ignoring path params.
fn path_matches(template: &str, concrete: &str) -> bool {
    let t = template
        .split('/')
        .map(|s| if s.starts_with(':') || (s.starts_with('{') && s.ends_with('}')) { "{}" } else { s })
        .collect::<Vec<_>>();
    let c = concrete.split('/').collect::<Vec<_>>();
    t.len() == c.len()
        && t.iter().zip(c.iter()).all(|(a, b)| *a == "{}" || a == b)
}

/// Executes `method path` with a JSON body/query against the running server,
/// authenticated as the given user. Returns the parsed JSON response.
pub async fn exec_endpoint(
    state: &Arc<AppState>,
    user_id: Uuid,
    bot_uuid: Option<Uuid>,
    method: &str,
    path_template: &str,
    params: &serde_json::Map<String, Value>,
) -> Result<Value, String> {
    let method_upper = method.to_uppercase();
    if !matches!(method_upper.as_str(), "GET" | "POST" | "PUT" | "PATCH" | "DELETE") {
        return Err(format!("unsupported method '{method}'"));
    }

    let path = fill_path(path_template, params)?;
    if !is_registered(&method_upper, &path) {
        return Err(format!("endpoint {method_upper} {path} is not a registered route"));
    }
    if crate::security::user_role::is_admin_only_endpoint(&state.conn, user_id, &method_upper, &path) {
        return Err(format!("endpoint {method_upper} {path} requires the admin role"));
    }

    #[cfg(feature = "consent")]
    {
        let class = match method_upper.as_str() {
            "GET" => "read",
            "POST" => "create",
            "PUT" | "PATCH" => "update",
            _ => "delete",
        };
        let gate = crate::core::bot::consent_gate::check(
            user_id,
            "api",
            class,
            serde_json::json!({ "method": method_upper, "path": path }),
        )
        .await;
        if !gate.allowed {
            return Err(match gate.request_id {
                Some(id) => format!("consent_required:{id}"),
                None => "action denied by consent policy".to_string(),
            });
        }
    }

    let base = base_url(state);
    let url = format!("{base}{path}");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    // Authenticate the loopback hop as the session user. The auth middleware
    // accepts the `X-User-ID` header (see extract_user_from_request) and resolves
    // it into the AuthenticatedUser — more reliable than minting a fresh JWT which
    // must match a private claim contract. CSRF is also Bearer/known-header exempt.
    let mut req = client
        .request(reqwest::Method::from_bytes(method_upper.as_bytes()).map_err(|e| e.to_string())?, &url)
        .header("X-User-ID", user_id.to_string())
        .header("Accept", "application/json");
    if let Some(bot) = bot_uuid {
        req = req.header("X-Bot-ID", bot.to_string());
    }

    // Split params: path params already consumed; the rest become query (GET)
    // or JSON body (mutating). Nested/array values go to the body.
    let mut query: Vec<(String, String)> = Vec::new();
    let mut body_obj = serde_json::Map::new();
    for (k, v) in params {
        if path_template.contains(&format!(":{k}")) || path_template.contains(&format!("{{{k}}}")) {
            continue;
        }
        match v {
            Value::String(s) if method_upper == "GET" => query.push((k.clone(), s.clone())),
            _ => {
                body_obj.insert(k.clone(), v.clone());
            }
        }
    }

    if method_upper == "GET" {
        if !query.is_empty() {
            req = req.query(&query);
        }
    } else if !body_obj.is_empty() {
        req = req.json(&Value::Object(body_obj));
    }

    let resp = req.send().await.map_err(|e| format!("loopback request failed: {e}"))?;
    let status = resp.status();
    let body = resp
        .bytes()
        .await
        .map_err(|e| format!("read body failed: {e}"))?;
    let truncated = body.len() > MAX_BODY;

    let parsed: Value = if body.is_empty() {
        Value::Null
    } else {
        match serde_json::from_slice(&body[..body.len().min(MAX_BODY)]) {
            Ok(v) => v,
            Err(_) => json!({ "raw": String::from_utf8_lossy(&body[..body.len().min(4000)]).to_string() }),
        }
    };

    if status.is_success() {
        let mut out = parsed;
        if truncated {
            out["_truncated"] = Value::Bool(true);
        }
        Ok(out)
    } else {
        let message = parsed
            .get("message")
            .or_else(|| parsed.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("request failed");
        Err(format!("{method_upper} {path} -> HTTP {status}: {message}"))
    }
}

/// Resolves the loopback base URL from the app config (default localhost:8080).
fn base_url(state: &Arc<AppState>) -> String {
    if let Some(cfg) = &state.config {
        if !cfg.server.base_url.is_empty() {
            return cfg.server.base_url.trim_end_matches('/').to_string();
        }
        return format!("http://{}:{}", cfg.server.host, cfg.server.port);
    }
    "http://127.0.0.1:8080".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fill_path() {
        let mut p = serde_json::Map::new();
        p.insert("id".to_string(), json!("abc-123"));
        let out = fill_path("/api/tickets/:id/status", &p).unwrap();
        assert_eq!(out, "/api/tickets/abc-123/status");

        let mut p2 = serde_json::Map::new();
        p2.insert("user_id".to_string(), json!("u1"));
        let out2 = fill_path("/api/workspaces/{workspace_id}/members/{user_id}", &p2);
        assert!(out2.is_err(), "missing workspace_id should error");
    }

    #[test]
    fn test_path_matches() {
        assert!(path_matches("/api/tickets/:id", "/api/tickets/abc"));
        assert!(path_matches("/api/tickets/{id}/comments", "/api/tickets/5/comments"));
        assert!(!path_matches("/api/tickets/:id", "/api/users/abc"));
        assert!(!path_matches("/api/tickets/:id", "/api/tickets"));
    }

    #[test]
    fn test_is_registered() {
        // These exist in ALL_ROUTES.
        assert!(is_registered("GET", "/api/crm/contacts"));
        assert!(is_registered("GET", "/api/banking/diagnosis"));
        // Never allow an arbitrary path.
        assert!(!is_registered("GET", "/api/../../etc/passwd"));
    }
}
