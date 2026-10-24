//! `projects_api::workspace_3` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

/// #1371 — Run for website projects: stage the workspace into the proxy
/// container's websites dir (no VM, no Incus resources) and return the site
/// URL. Mirrors the publish path's two-sites-per-project model: every Run
/// refreshes the TEST twin so an edit under test can never blank the live
/// page; the public slug is only written when the caller carries the
/// sanctioned production approval (same guard as `publish_project`).
pub(crate) async fn run_website_via_proxy(
    registry: &ProjectRegistryRef,
    project: &crate::projects::Project,
) -> (StatusCode, Json<serde_json::Value>) {
    // Reuse the publish path verbatim: it enforces metering, resolves the
    // site env with the production approval guard (Run without the deploy
    // stamp always lands on the test twin), stages the release, records the
    // deployment history and returns the site URL. Run defaults to `test` —
    // a preview must never touch the live slug.
    let args = serde_json::json!({ "project_id": project.id.to_string(), "env": "test" });
    match crate::publish::do_publish(args, registry.pool().clone()).await {
        Ok(data) => {
            let url = data
                .get("url")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let env = data
                .get("env")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("test")
                .to_string();
            let note = data
                .get("note")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "url": url,
                    "host_url": url,
                    "container": "proxy",
                    "deploy_target": "proxy-websites",
                    "env": env,
                    "note": note,
                    "vm": serde_json::Value::Null,
                    "project": project.name,
                })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "success": false, "error": e })),
        ),
    }
}

/// #1271 — same-origin preview of the project's dev VM. `run` starts the app
/// as a real process in the container and exposes it through a host proxy
/// device (`localhost:{port}`); the generic browser proxy refuses private
/// hosts, so this route fetches the dev-VM host address server-side and
/// streams the response back — authenticated exactly like the workspace
/// `serve` route (Bearer header from the desktop, or `?token=` from the
/// embedded iframe). Only ports from the dev-VM range are accepted; RBAC
/// still gates access.
pub(crate) async fn preview_vm_app(
    Extension(registry): Extension<ProjectRegistryRef>,
    Extension(rbac): Extension<ProjectRbac>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(project_id): Path<Uuid>,
    Query(query): Query<VmPreviewQuery>,
    method: axum::http::Method,
    request_headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    // #1340 — same iframe transport as `serve_project_file` (see
    // `resolve_iframe_user`): header-less requests authenticate via `?token=`.
    let user = if user.is_authenticated() {
        user
    } else {
        match resolve_iframe_user(query.token.clone()) {
            Some(u) => u,
            None => return (StatusCode::UNAUTHORIZED, "unauthorized").into_response(),
        }
    };
    if let Err(e) = rbac.require_role(user.user_id, project_id, ProjectRole::Viewer) {
        log::warn!("Vibe vm-preview forbidden: {e}");
        return (StatusCode::FORBIDDEN, "forbidden").into_response();
    }
    let project = match registry.get(project_id) {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "project not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let port = query.port;
    if !(DEV_VM_PORT_MIN..DEV_VM_PORT_MAX).contains(&port) {
        log::warn!("Vibe vm-preview {}: out-of-range port {port}", project.name);
        return (StatusCode::BAD_REQUEST, "invalid dev-vm port").into_response();
    }
    let path = query.path.as_deref().unwrap_or("/");
    // The vibe-http proxy device binds on the Incus HOST (`0.0.0.0:{port}`).
    // When botserver runs directly on the host (dev machines) `127.0.0.1` is
    // correct; when botserver runs inside an Incus container (prod bot
    // container), 127.0.0.1 is the container itself and the host is only
    // reachable via the default gateway. Probe the candidates in order.
    let mut candidates = vec![format!("http://127.0.0.1:{port}{path}")];
    if let Some(gateway) = default_gateway_ip() {
        let host_candidate = format!("http://{gateway}:{port}{path}");
        if !candidates.contains(&host_candidate) {
            candidates.push(host_candidate);
        }
    }
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Vibe vm-preview {}: client build failed: {e}", project.name);
            return (StatusCode::INTERNAL_SERVER_ERROR, "proxy client").into_response();
        }
    };
    // The preview proxies the app's own HTTP surface: a dev app with
    // POST/PUT/DELETE endpoints must work inside the embedded Browser, so the
    // request method, body and content type are forwarded (a GET-only route
    // answered 405 for every mutation the app made through the shim).
    let upstream_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(m) => m,
        Err(_) => reqwest::Method::GET,
    };
    let forwards_body = !body.is_empty() && method != axum::http::Method::HEAD;
    let content_type = request_headers
        .get(axum::http::header::CONTENT_TYPE)
        .map(|v| v.as_bytes().to_vec());
    let mut last_error: Option<(String, String)> = None;
    let mut resp = None;
    for target in &candidates {
        let mut request = client.request(upstream_method.clone(), target);
        if forwards_body {
            request = request.body(body.clone());
        }
        if let Some(ct) = &content_type {
            request = request.header(reqwest::header::CONTENT_TYPE, ct.clone());
        }
        match request.send().await {
            Ok(r) => {
                resp = Some(r);
                break;
            }
            Err(e) => last_error = Some((target.clone(), e.to_string())),
        }
    }
    let resp = match resp {
        Some(r) => r,
        None => {
            // Every candidate failed; log the last attempt's target and error.
            let (target, err) = last_error
                .unwrap_or_else(|| (candidates[0].clone(), "no reachable candidate".to_string()));
            log::warn!("Vibe vm-preview {}: fetch {target} failed: {err}", project.name);
            return (StatusCode::BAD_GATEWAY, "dev vm not reachable").into_response();
        }
    };
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            log::warn!("Vibe vm-preview {}: read response: {e}", project.name);
            return (StatusCode::BAD_GATEWAY, "read dev vm response").into_response();
        }
    };
    let body: axum::body::Body = if content_type.to_lowercase().contains("html") {
        match (std::str::from_utf8(&bytes), query.token.as_deref()) {
            (Ok(text), Some(token)) => serve_inject_preview(text, token, project_id, port)
                .into_bytes()
                .into(),
            _ => bytes.into(),
        }
    } else {
        bytes.into()
    };
    (
        status,
        [
            (axum::http::header::CONTENT_TYPE, content_type),
            (axum::http::header::CACHE_CONTROL, "no-store".to_string()),
        ],
        body,
    )
        .into_response()
}

// ── #1192: serve the project's own custom app (workspace static preview) ──
// Play/Preview resolves a project to this route when its workspace has source
// files, so the LLM-generated app runs in the Browser window without needing a
// VM. Files are streamed from `VIBE_WORKSPACE_ROOT/{slug}/` with the correct
// MIME type; HTML responses have relative asset URLs rewritten to carry the
// auth token (iframes cannot set headers).
/// #1340 — resolve the iframe-auth `?token=` capability for the two
/// same-origin Vibe routes (`serve`, `vm-preview`). A browser iframe cannot
/// set an Authorization header, and the auth middleware runs before the
/// handler, so an embedded preview arrives anonymous and gets `missing_token`
/// before RBAC is ever consulted. The handler therefore re-runs the
/// middleware's own session-cache lookup (`botsecurity_core::lookup_session_cache`,
/// the same capability store the terminal WS gate uses) and rebuilds the
/// `AuthenticatedUser` from the cached entry. Tokens are opaque `gb_*` session
/// ids: every invalid value resolves to `None` and is rejected — the route is
/// never open, it just accepts the header-less transport.
pub(crate) fn resolve_iframe_user(token: Option<String>) -> Option<AuthenticatedUser> {
    let token = token?;
    if token.trim().is_empty() {
        return None;
    }
    let entry = botsecurity_core::lookup_session_cache(&token)?;
    let user_id = uuid::Uuid::parse_str(&entry.user_id).unwrap_or_else(|_| {
        uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_DNS,
            format!("zitadel:{}", entry.user_id).as_bytes(),
        )
    });
    let mut user = AuthenticatedUser::new(user_id, entry.email.clone()).with_session(token);
    for role_str in &entry.roles {
        user = user.with_role(match role_str.to_lowercase().as_str() {
            "admin" | "administrator" => Role::Admin,
            "superadmin" | "super_admin" => Role::SuperAdmin,
            "moderator" => Role::Moderator,
            "bot_owner" => Role::BotOwner,
            "bot_operator" => Role::BotOperator,
            "bot_viewer" => Role::BotViewer,
            "service" => Role::Service,
            _ => Role::User,
        });
    }
    if entry.roles.is_empty() {
        user = user.with_role(Role::User);
    }
    if let Some(org) = entry.organization_id {
        user = user.with_organization(org);
    }
    Some(user)
}

/// Rewrite an HTML document served through the dev-VM preview proxy so the app
/// keeps working when embedded in a same-origin iframe.
///
/// Two problems are solved (reported 2026-09-01):
/// 1. Root-relative `src`/`href` URLs (`/style.css`) and inline `fetch('/api/..')`
///    calls resolve against the botserver origin, hit the `/api/*` auth middleware
///    and return `missing_token`. They are rewritten to route back through the
///    preview proxy itself (`path=` + `token=`), exactly like the workspace
///    `serve` route does for relative assets.
/// 2. Relative asset URLs (`app.js`) are also routed through the proxy (they
///    would otherwise resolve against `/api/vibe/projects/...` and 404).
pub(crate) fn serve_inject_preview(html: &str, token: &str, project_id: Uuid, port: u16) -> String {
    if token.is_empty() {
        return html.to_string();
    }
    let proxy_base = format!(
        "/api/vibe/projects/{project_id}/vm-preview?port={port}&path="
    );
    let shim = format!(
        r##"<script>/* gb vm-preview proxy shim */
(function(){{
  var base = {proxy_base_q:?};
  var token = {token_q:?};
  function proxify(u) {{
    if (typeof u !== "string" || !u) return u;
    if (u.indexOf("http://") === 0 || u.indexOf("https://") === 0 ||
        u.indexOf("//") === 0 || u.indexOf("data:") === 0 ||
        u.indexOf("blob:") === 0 || u.indexOf("#") === 0 ||
        u.indexOf("/api/vibe/projects/") === 0) return u;
    var p = u.charAt(0) === "/" ? u : "/" + u;
    return base + encodeURIComponent(p) + "&token=" + token;
  }}
  var of = window.fetch;
  if (of) window.fetch = function(input, init) {{
    return of.call(this, proxify(input), init);
  }};
  var OX = window.XMLHttpRequest;
  if (OX && OX.prototype && OX.prototype.open) {{
    var op = OX.prototype.open;
    OX.prototype.open = function(m, u, async, user, pass) {{
      return op.call(this, m, proxify(u), async, user, pass);
    }};
  }}
}})();</script>"##,
        proxy_base_q = proxy_base,
        token_q = token,
    );
    let mut out = String::with_capacity(html.len() + shim.len() + 64);
    let mut rest = html;
    while !rest.is_empty() {
        // Find the next `src="|href="|src='|href='` marker.
        let candidates = ["src=\"", "href=\"", "src='", "href='"]
            .into_iter()
            .filter_map(|m| rest.find(m).map(|p| (p, m)))
            .min_by_key(|(p, _)| *p);
        match candidates {
            Some((pos, marker)) => {
                out.push_str(&rest[..pos]);
                out.push_str(marker);
                let quote = &marker[marker.len() - 1..];
                let value_start = pos + marker.len();
                let value_end = rest[value_start..]
                    .find(quote)
                    .map(|p| value_start + p)
                    .unwrap_or(rest.len());
                let url = &rest[value_start..value_end];
                if !url.is_empty()
                    && !url.starts_with("http://")
                    && !url.starts_with("https://")
                    && !url.starts_with("//")
                    && !url.starts_with("#")
                    && !url.starts_with("data:")
                    && !url.starts_with("blob:")
                    && !url.starts_with("/api/vibe/projects/")
                {
                    let p = if url.starts_with('/') {
                        url.to_string()
                    } else {
                        format!("/{url}")
                    };
                    out.push_str(&proxy_base);
                    out.push_str(&urlencode_path(&p));
                    out.push_str("&token=");
                    out.push_str(token);
                } else {
                    out.push_str(url);
                }
                out.push_str(quote);
                rest = &rest[value_end + 1..];
            }
            None => {
                out.push_str(rest);
                rest = "";
            }
        }
    }
    // Inject the fetch/XHR shim before </head> (or prepend if no head).
    if let Some(idx) = out.find("</head>") {
        out.insert_str(idx, &shim);
    } else {
        out.insert_str(0, &shim);
    }
    out
}
