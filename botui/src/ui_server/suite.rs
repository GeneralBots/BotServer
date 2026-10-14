use axum::{
    extract::{OriginalUri, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use log::{error, info, warn};
use serde::Deserialize;
use std::fs;

use crate::shared::AppState;
#[cfg(feature = "embed-ui")]
use crate::ui_server::constants::Assets;
use crate::ui_server::constants::get_ui_root;
use crate::ui_server::suite_ops::remove_section;

#[derive(Deserialize)]
pub struct SuiteQueryParams {
    pub bot_name: Option<String>,
}

/// Resolve a hostname to a bot name via the botserver domain API.
async fn resolve_bot_from_host(state: &AppState, host: &str) -> Option<String> {
    let resolve_url = format!("{}/api/domains/resolve?host={}", state.client.base_url(), urlencoding(host));
    let client = crate::ui_server::tls_policy::backend_http_client(state.client.base_url());
    match client.get(&resolve_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                let found = data.get("found").and_then(|v| v.as_bool()).unwrap_or(false);
                if found {
                    return data.get("bot_name").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
            }
            None
        }
        _ => None,
    }
}

fn urlencoding(s: &str) -> String {
    s.chars().fold(String::new(), |mut acc, c| {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => acc.push(c),
            _ => acc.push_str(&format!("%{:02X}", c as u32)),
        }
        acc
    })
}

/// Platform domain for SaaS subdomain mode (parameterized for self-hosting).
/// `{bot}.{platform}` serves the bot named `{bot}`; override with the
/// `GB_PLATFORM_DOMAIN` env var when self-hosting under a different domain.
fn platform_domain() -> String {
    std::env::var("GB_PLATFORM_DOMAIN")
        .unwrap_or_else(|_| "generalbots.org".to_string())
        .to_lowercase()
}

/// True when `host` is the platform apex or any subdomain of it.
fn is_platform_host(host: &str) -> bool {
    let d = platform_domain();
    host == d || host.ends_with(&format!(".{d}"))
}

/// Extract the bot name from a `{bot}.{platform}` host.
/// `None` for the apex and reserved service hosts (chat., www., …) —
/// those serve the default bot.
fn platform_subdomain_bot(host: &str) -> Option<String> {
    let d = platform_domain();
    let sub = host.strip_suffix(&format!(".{d}"))?.to_lowercase();
    const RESERVED: &[&str] = &[
        "chat", "www", "api", "app", "bot", "docs", "store", "cloud", "login",
        "admin", "mail", "smtp", "imap", "ns1", "ns2",
    ];
    if sub.is_empty() || RESERVED.contains(&sub.as_str()) {
        return None;
    }
    Some(sub)
}

pub async fn index(
    State(state): State<AppState>,
    OriginalUri(uri): OriginalUri,
    headers: axum::http::HeaderMap,
) -> Response {
    let path = uri.path();

    // Check Host header for domain-based bot resolution
    let domain_resolved_bot = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(|host| {
            // Strip port if present (e.g., "chat.generalbots.org:3000" -> "chat.generalbots.org")
            let host_clean = host.split(':').next().unwrap_or(host);
            if host_clean.contains('.') && !host_clean.starts_with("localhost") {
                Some(host_clean.to_lowercase())
            } else {
                None
            }
        });

    let path_parts: Vec<&str> = path.split('/').collect();
    let mut bot_name = path_parts
        .iter()
        .rev()
        .find(|part| {
            !part.is_empty()
                && **part != "chat"
                && **part != "app"
                && **part != "ws"
                && **part != "ui"
                && **part != "api"
                && **part != "auth"
                && **part != "suite"
                && **part != "cloud"
                && !part.ends_with(".js")
                && !part.ends_with(".css")
        })
        .map(|s| s.to_string());

    // If no bot in URL path and we have a domain, resolve it. Platform
    // subdomains ({bot}.{GB_PLATFORM_DOMAIN}) resolve locally with no API
    // call; custom domains go through the botserver domain API.
    if bot_name.is_none() {
        if let Some(ref host) = domain_resolved_bot {
            if let Some(sub) = platform_subdomain_bot(host) {
                bot_name = Some(sub);
            } else if let Some(resolved) = resolve_bot_from_host(&state, host).await {
                bot_name = Some(resolved);
            }
        }
    }

    // Suite directories that can be accessed directly without bot auth
    let suite_dirs: &[&str] = &["drive", "chat", "tasks", "admin", "mail", "calendar", "meet", "docs", "sheet", "slides", "paper", "research", "learn", "analytics", "dashboards", "monitoring", "governance", "people", "crm", "tickets", "billing", "products", "video", "player", "canvas", "social", "project", "goals", "workspace", "designer", "vibe", "integrations", "erp", "fraud", "settings", "about", "tools", "attendant", "banking", "biometry", "brazil", "browser", "campaigns", "compliance", "database", "desktop", "email", "handoff", "hr", "itsm", "kyc", "lists", "o365", "minutes", "plan", "plugins", "pos", "retail", "sales", "tax", "templates", "templates-app", "terminal", "timeclock", "vision"];
    let known_dirs: &[&str] = &["suite", "js", "css", "vendor", "assets", "public", "partials", "settings", "about", "drive", "chat", "tasks", "admin", "mail", "calendar", "meet", "docs", "sheet", "slides", "paper", "research", "learn", "analytics", "dashboards", "monitoring", "governance", "people", "crm", "tickets", "billing", "products", "video", "player", "canvas", "social", "project", "goals", "workspace", "designer", "vibe", "integrations", "erp", "fraud", "attendant", "banking", "biometry", "brazil", "browser", "campaigns", "compliance", "database", "desktop", "email", "handoff", "hr", "itsm", "kyc", "lists", "o365", "minutes", "plan", "plugins", "pos", "retail", "sales", "tax", "templates", "templates-app", "terminal", "timeclock", "tools", "vision"];
    let is_suite_dir = bot_name.as_ref().is_some_and(|b| suite_dirs.contains(&b.as_str()));

    // Suite dirs (vibe, project, database, …) are apps, not bots. Resolve the
    // real bot from the domain: platform subdomains ({bot}.{GB_PLATFORM_DOMAIN})
    // resolve locally (apex/reserved hosts serve "default"); custom domains go
    // through the domain API with a subdomain-parts fallback
    // (e.g. chat.pragmatismo.com.br -> pragmatismo when bot_domains is empty).
    if is_suite_dir {
        let mut resolved = None;
        if let Some(ref host) = domain_resolved_bot {
            if is_platform_host(host) {
                resolved = Some(platform_subdomain_bot(host).unwrap_or_else(|| "default".to_string()));
            } else {
                resolved = resolve_bot_from_host(&state, host).await;
                if resolved.is_none() {
                    // Fallback: derive the bot from the chat subdomain, e.g.
                    // chat.pragmatismo.com.br -> pragmatismo (bot_domains may be empty).
                    let parts: Vec<&str> = host.split('.').collect();
                    if parts.len() >= 2 {
                        resolved = match parts[0] {
                            "chat" | "www" | "bot" | "app" => Some(parts[1].to_string()),
                            _ => Some(parts[0].to_string()),
                        };
                    }
                }
            }
        }
        bot_name = Some(resolved.unwrap_or_else(|| "default".to_string()));
    }

    if let Some(ref bot) = bot_name {
        if !is_suite_dir {
        // Check for token from query param (cloud dashboard link)
        let query_token = uri.query().and_then(|q| {
            q.split('&').find_map(|p| {
                let mut parts = p.splitn(2, '=');
                match (parts.next(), parts.next()) {
                    (Some("token"), Some(v)) => Some(v.to_string()),
                    _ => None,
                }
            })
        });
        let query_token_present = query_token.is_some();
        let cookie_token = headers
            .get(axum::http::header::COOKIE)
            .and_then(|c| c.to_str().ok())
            .and_then(|c| {
                c.split(';')
                    .filter_map(|part| part.trim().split_once('='))
                    .find(|(name, _)| *name == "gb-access-token")
                    .map(|(_, value)| value.to_string())
            })
            .filter(|value| !value.is_empty());
        let auth_header = query_token
            .map(|t| format!("Bearer {}", t))
            .or_else(|| {
                headers.get(axum::http::header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .or_else(|| cookie_token.map(|t| format!("Bearer {}", t)));

        let target_url = format!("{}/api/bots/{}/access", state.client.base_url(), bot);
        let client = crate::ui_server::tls_policy::backend_http_client(state.client.base_url());

        let mut req = client.get(&target_url);
        if let Some(ref auth) = auth_header {
            req = req.header(axum::http::header::AUTHORIZATION, auth);
        }
        for (k, v) in headers.iter() {
            if k != axum::http::header::HOST {
                req = req.header(k, v);
            }
        }

        // Only redirect to login when a token was actually supplied but
        // rejected. Anonymous callers (or sessions stored client-side in
        // localStorage after the suite-sso hop) must be served the page —
        // their JS attaches the token to API calls. Redirecting them here
        // would loop: SSO hop -> clean URL -> access check sees no token ->
        // login -> ...
        let has_token = headers
            .get(axum::http::header::COOKIE)
            .and_then(|c| c.to_str().ok())
            .is_some_and(|s| s.contains("gb-access-token"))
            || query_token_present;

        match req.send().await {
            Ok(resp) => {
                // #1435 — a 401 with NO token means the bot is PRIVATE and
                // the caller is anonymous: the desktop/chat shell must not
                // be served at all (previously only token-holders were
                // redirected, so anonymous visitors saw the anon-limited
                // shell of a private bot). A 401 WITH a token is a stale
                // credential — also redirect. 403 is a signed-in user the
                // bot does not belong to: clean 403 page, not the shell.
                if resp.status() == axum::http::StatusCode::UNAUTHORIZED
                    || resp.status() == axum::http::StatusCode::FORBIDDEN
                {
                    if resp.status() == axum::http::StatusCode::FORBIDDEN && !has_token {
                        return (
                            axum::http::StatusCode::FORBIDDEN,
                            "<!doctype html><html><body style=\"font-family:sans-serif;display:flex;align-items:center;justify-content:center;height:100vh;margin:0\"><div style=\"text-align:center\"><h2>403 \u{2014} Access denied</h2><p>You do not have access to this bot.</p></div></body></html>",
                        )
                            .into_response();
                    }
                    info!("index: Access denied for bot {} (redirecting to login)", bot);
                    let login_url = std::env::var("LOGIN_URL")
                        .unwrap_or_else(|_| "http://localhost:5000".to_string());
                    // Return to this exact page after login: the login page
                    // resolves the absolute `redirect` param and lands back
                    // here with a fresh token. Strip any stale token/email/name
                    // params from the return URL to avoid a login loop.
                    let host = headers
                        .get(axum::http::header::HOST)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    let scheme = if host.contains("localhost") || host.contains("127.0.0.1") {
                        "http"
                    } else {
                        "https"
                    };
                    let clean_query = uri
                        .query()
                        .unwrap_or("")
                        .split('&')
                        .filter(|p| {
                            !p.starts_with("token=")
                                && !p.starts_with("email=")
                                && !p.starts_with("name=")
                        })
                        .collect::<Vec<_>>()
                        .join("&");
                    let return_url = if clean_query.is_empty() {
                        format!("{scheme}://{host}{}", uri.path())
                    } else {
                        format!("{scheme}://{host}{}?{clean_query}", uri.path())
                    };
                    // #1324 — never point the login redirect at an auth page.
                    // When the gated path is itself /login or /signup the
                    // redirect chain nests `?redirect=` on every hop until the
                    // URL explodes and the browser gives up ("can't be
                    // reached"). Auth pages return to the default destination
                    // instead of asking to return to themselves.
                    let path_is_auth = {
                        let p = uri.path().trim_end_matches('/').to_lowercase();
                        p.ends_with("/login") || p.ends_with("/signup") || p == "/login" || p == "/signup"
                    };
                    if path_is_auth {
                        return Redirect::to(&login_url).into_response();
                    }
                    return Redirect::to(&format!(
                        "{}?redirect={}",
                        login_url,
                        urlencoding(&return_url)
                    ))
                    .into_response();
                }
            }
            Err(e) => {
                warn!("index: Access check failed for bot {}: {}", bot, e);
            }
        }
        }
    }

    let path_lower = path.to_lowercase();

    // Cloud pages are served exclusively on port 4000 — never on the suite port
    if path_lower.starts_with("/cloud/") || path_lower == "/cloud" {
        return StatusCode::NOT_FOUND.into_response();
    }

    // Auth pages are served exclusively on port 5000 — never on the suite port
    if path_lower.starts_with("/auth/") || path_lower == "/auth" {
        return StatusCode::NOT_FOUND.into_response();
    }

    // Check if path contains static asset directories - serve them directly
    if path_lower.contains("/js/")
        || path_lower.contains("/css/")
        || path_lower.contains("/vendor/")
        || path_lower.contains("/assets/")
        || path_lower.contains("/public/")
        || path_lower.contains("/partials/")
        || path_lower.contains("/crm/")
        || path_lower.contains("/tasks/")
        || path_lower.contains("/drive/")
        || path_lower.contains("/terminal/")
        || path_lower.contains("/browser/")
        || path_lower.contains("/editor/")
        || path_lower.ends_with(".js")
        || path_lower.ends_with(".css")
        || path_lower.ends_with(".html")
        || path_lower.ends_with(".png")
        || path_lower.ends_with(".jpg")
        || path_lower.ends_with(".jpeg")
        || path_lower.ends_with(".gif")
        || path_lower.ends_with(".svg")
        || path_lower.ends_with(".ico")
        || path_lower.ends_with(".woff")
        || path_lower.ends_with(".woff2")
        || path_lower.ends_with(".ttf")
        || path_lower.ends_with(".eot")
        || path_lower.ends_with(".mp4")
        || path_lower.ends_with(".webm")
        || path_lower.ends_with(".mp3")
        || path_lower.ends_with(".wav")
    {
        let path_parts: Vec<&str> = path.split('/').collect();
        let fs_path = if path_parts.len() > 1 {
            let mut start_idx = 1;

            if known_dirs.contains(&path_parts[1]) {
                if suite_dirs.contains(&path_parts[1]) {
                    format!("suite/{}", path_parts[1..].join("/"))
                } else {
                    path_parts[1..].join("/")
                }
            } else {
                if path_parts.len() > start_idx + 1
                    && known_dirs.contains(&path_parts[start_idx + 1])
                {
                    start_idx += 1;
                }

                path_parts[start_idx..].join("/")
            }
        } else {
            path.to_string()
        };

        let full_path = get_ui_root().join(&fs_path);

        info!("index: Serving static file: {} -> {:?} (fs_path: {})", path, full_path, fs_path);

        #[cfg(feature = "embed-ui")]
        {
            let asset_path = fs_path.trim_start_matches('/');
            if let Some(content) = Assets::get(asset_path) {
                let mime = mime_guess::from_path(asset_path).first_or_octet_stream();
                return ([(axum::http::header::CONTENT_TYPE, mime.as_ref())], content.data).into_response();
            }
        }

        #[cfg(not(feature = "embed-ui"))]
        {
            if let Ok(bytes) = tokio::fs::read(&full_path).await {
                let mime = mime_guess::from_path(&full_path).first_or_octet_stream();
                let content_type = if mime.as_ref() == "text/html" { "text/html; charset=utf-8" } else { mime.as_ref() };
                return (StatusCode::OK, [("content-type", content_type)], bytes).into_response();
            }
            // Directory index: serve index.html for directories
            if full_path.is_dir() {
                let index_path = full_path.join("index.html");
                if let Ok(bytes) = tokio::fs::read(&index_path).await {
                    return (StatusCode::OK, [("content-type", "text/html; charset=utf-8")], bytes).into_response();
                }
            }
        }

        warn!("index: Static file not found: {} -> {:?}", path, full_path);
        return StatusCode::NOT_FOUND.into_response();
    }

    if bot_name.is_none() {
        if let Some(ref host) = domain_resolved_bot {
            if let Some(sub) = platform_subdomain_bot(host) {
                bot_name = Some(sub);
            } else if let Some(resolved) = resolve_bot_from_host(&state, host).await {
                bot_name = Some(resolved);
            }
        }
    }

    info!(
        "index: Extracted bot_name: {:?} from path: {}",
        bot_name,
        path
    );
    serve_suite_impl(&state, bot_name, headers.clone()).await
}

pub async fn serve_suite(
    State(state): State<AppState>,
    Query(params): Query<SuiteQueryParams>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    serve_suite_impl(&state, params.bot_name, headers).await
}

pub async fn serve_suite_impl(_state: &AppState, bot_name: Option<String>, _headers: axum::http::HeaderMap) -> Response {
    let is_auth = bot_name.as_ref()
        .map(|n| n.ends_with(".html") || n == "login" || n == "register" || n == "forgot-password" || n == "reset-password")
        .unwrap_or(false);

    if !is_auth {
        // Access control is handled by the botserver at WebSocket level.
        // The UI always renders and lets the botserver enforce auth.
    }
    let raw_html_res = {
        #[cfg(feature = "embed-ui")]
        {
            match Assets::get("suite/desktop.html") {
                Some(f) => String::from_utf8(f.data.into_owned()).map_err(|e| e.to_string()),
                None => {
                    let path = get_ui_root().join("suite/desktop.html");
                    log::warn!("Asset .suite/desktop.html. not found in embedded binary, falling back to filesystem: {:?}", path);
                    fs::read_to_string(&path).map_err(|e| {
                        format!(
                            "Asset not found in binary AND failed to read {:?} (CWD: {:?}): {}",
                            path,
                            std::env::current_dir(),
                            e
                        )
                    })
                }
            }
        }
        #[cfg(not(feature = "embed-ui"))]
        {
            let path = get_ui_root().join("suite/desktop.html");
            fs::read_to_string(&path).map_err(|e| {
                format!(
                    "Failed to read {:?} (CWD: {:?}): {}",
                    path,
                    std::env::current_dir(),
                    e
                )
            })
        }
    };

    match raw_html_res {
        Ok(raw_html) => {
            let mut html = raw_html;
            let _ = &mut html ;

            if let Some(head_end) = html.find("</head>") {
                let is_auth_page = bot_name.as_ref()
                    .map(|n| n.ends_with(".html") || n == "login" || n == "register" || n == "forgot-password" || n == "reset-password")
                    .unwrap_or(false);

                let base_href = if is_auth_page {
                    "/".to_string()
                } else if let Some(ref name) = bot_name {
                    format!("/{}/", name)
                } else {
                    "/".to_string()
                };
                let base_tag = format!(r#"<base href="{}">"#, base_href);
                html.insert_str(head_end, &base_tag);

                if !is_auth_page {
                    let login_url = std::env::var("LOGIN_URL")
                        .unwrap_or_else(|_| "http://localhost:5000".to_string());
                    let login_script = format!(
                        r#"<script>window.GB_LOGIN_URL = "{}";</script>"#,
                        login_url
                    );
                    html.insert_str(head_end + base_tag.len(), &login_script);

                    if let Some(name) = bot_name {
                        let ins_offset = head_end + base_tag.len() + login_script.len();
                        info!("serve_suite: Injecting bot_name '{}' into page with base href='{}'", name, base_href);
                        let bot_script = format!(
                            r#"<script>window.__INITIAL_BOT_NAME__ = "{}";</script>"#,
                            name
                        );
                        html.insert_str(ins_offset, &bot_script);
                        info!("serve_suite: Successfully injected base tag and bot_name script");
                    } else {
                        info!("serve_suite: Successfully injected base tag (no bot_name)");
                    }
                } else {
                    info!("serve_suite: Auth page detected, skipping bot_name injection (base href='{}')", base_href);
                }
            } else {
                error!("serve_suite: Failed to find </head> tag to inject content");
            }

            // Core Apps
            #[cfg(not(feature = "chat"))]
            {
                html = remove_section(&html, "chat");
            }
            #[cfg(not(feature = "mail"))]
            {
                html = remove_section(&html, "mail");
            }
            #[cfg(not(feature = "calendar"))]
            {
                html = remove_section(&html, "calendar");
            }
            #[cfg(not(feature = "drive"))]
            {
                html = remove_section(&html, "drive");
            }
            #[cfg(not(feature = "tasks"))]
            {
                html = remove_section(&html, "tasks");
            }
            #[cfg(not(feature = "meet"))]
            {
                html = remove_section(&html, "meet");
            }

            // Documents
            #[cfg(not(feature = "docs"))]
            {
                html = remove_section(&html, "docs");
            }
            #[cfg(not(feature = "sheet"))]
            {
                html = remove_section(&html, "sheet");
            }
            #[cfg(not(feature = "slides"))]
            {
                html = remove_section(&html, "slides");
            }
            #[cfg(not(feature = "paper"))]
            {
                html = remove_section(&html, "paper");
            }

            // Research
            #[cfg(not(feature = "research"))]
            {
                html = remove_section(&html, "research");
            }
            #[cfg(not(feature = "learn"))]
            {
                html = remove_section(&html, "learn");
            }

            // Analytics
            #[cfg(not(feature = "analytics"))]
            {
                html = remove_section(&html, "analytics");
            }
            #[cfg(not(feature = "dashboards"))]
            {
                html = remove_section(&html, "dashboards");
            }
            #[cfg(not(feature = "monitoring"))]
            {
                html = remove_section(&html, "monitoring");
                html = remove_section(&html, "governance");
            }

            // Business
            #[cfg(not(feature = "people"))]
            {
                html = remove_section(&html, "people");
                html = remove_section(&html, "crm");
            }
            #[cfg(not(feature = "billing"))]
            {
                html = remove_section(&html, "billing");
            }
            #[cfg(not(feature = "products"))]
            {
                html = remove_section(&html, "products");
            }
            #[cfg(not(feature = "tickets"))]
            {
                html = remove_section(&html, "tickets");
            }

            // Media
            #[cfg(not(feature = "video"))]
            {
                html = remove_section(&html, "video");
            }
            #[cfg(not(feature = "player"))]
            {
                html = remove_section(&html, "player");
            }
            #[cfg(not(feature = "canvas"))]
            {
                html = remove_section(&html, "canvas");
            }

            // Social & Project
            #[cfg(not(feature = "social"))]
            {
                html = remove_section(&html, "social");
            }
            #[cfg(not(feature = "project"))]
            {
                html = remove_section(&html, "project");
            }
            #[cfg(not(feature = "goals"))]
            {
                html = remove_section(&html, "goals");
            }
            #[cfg(not(feature = "workspace"))]
            {
                html = remove_section(&html, "workspace");
            }

            // Admin/Tools
            #[cfg(not(feature = "admin"))]
            {
                html = remove_section(&html, "admin");
            }
            #[cfg(not(feature = "tools"))]
            {
                html = remove_section(&html, "security");
            }
            #[cfg(not(feature = "attendant"))]
            {
                html = remove_section(&html, "attendant");
            }
            #[cfg(not(feature = "designer"))]
            {
                html = remove_section(&html, "designer");
            }
            #[cfg(not(feature = "editor"))]
            {
                html = remove_section(&html, "editor");
            }
            #[cfg(not(feature = "settings"))]
            {
                html = remove_section(&html, "settings");
            }

            (StatusCode::OK, [("content-type", "text/html; charset=utf-8"), ("cache-control", "no-cache, no-store, must-revalidate"), ("pragma", "no-cache"), ("expires", "0")], Html(html)).into_response()
        }
        Err(e) => {
            error!("Failed to load suite UI: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                Html("Failed to load suite interface".to_string()),
            ).into_response()
        }
    }
}


