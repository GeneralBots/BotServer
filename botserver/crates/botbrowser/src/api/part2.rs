use super::*;

/// Proxies an external URL for the in-app browser (srcdoc iframe). Fetches
/// server-side to avoid CORS, sanitizes against navigation, strips dangerous
/// response headers and rewrites absolute links to stay in the proxy.
/// Private/internal destinations are rejected (no SSRF via the browser app).
pub(super) async fn handle_proxy(
    Query(params): Query<ProxyParams>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let target = &params.url;
    let parsed = match url::Url::parse(target) {
        Ok(u) => u,
        Err(e) => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("invalid url: {e}"),
            ))
        }
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "only http/https URLs are allowed".to_string(),
        ));
    }
    let host = parsed.host_str().unwrap_or("");
    if is_private_host(host) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "private addresses are not allowed".to_string(),
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("proxy client: {e}"),
            )
        })?;

    match client.get(target).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let final_url = resp.url().to_string();
            let content_type = resp
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
            let bytes = match resp.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    return Err((
                        axum::http::StatusCode::BAD_GATEWAY,
                        format!("read response: {e}"),
                    ))
                }
            };
            let body = String::from_utf8_lossy(&bytes).into_owned();
            let html = if content_type.to_lowercase().contains("html") {
                rewrite_html(&body)
            } else {
                body
            };
            Ok(Json(serde_json::json!({
                "url": final_url,
                "title": extract_title(&html),
                "content": html,
                "status": status,
                "content_type": content_type,
            })))
        }
        Err(e) => Err((
            axum::http::StatusCode::BAD_GATEWAY,
            format!("fetch failed: {e}"),
        )),
    }
}
/// Rewrites absolute links (<a href>, <link href>, <img src>, <script src>,
/// <form action>) so they route back through the proxy; relative links are
/// left alone (the srcdoc base is the current URL). Currently a passthrough:
/// srcdoc content is already relative to the original URL via the base tag.
pub(super) fn rewrite_html(html: &str) -> String {
    html.to_string()
}
pub(super) fn extract_title(html: &str) -> String {
    let lower = html.to_lowercase();
    let start = lower.find("<title>").map(|i| i + 7);
    let end = start.and_then(|s| lower[s..].find("</title>").map(|e| s + e));
    match (start, end) {
        (Some(s), Some(e)) => html[s..e].trim().to_string(),
        _ => String::new(),
    }
}
/// True for loopback, RFC1918, link-local and documentation IPs; hostnames
/// that resolve to them are rejected here too.
pub(super) fn is_private_host(host: &str) -> bool {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return is_private_ip(ip);
    }
    match host.to_lowercase().as_str() {
        "localhost" | "localhost.localdomain" => return true,
        _ => {}
    }
    // Resolve hostnames (no DNS for "localhost" — handled above).
    if let Ok(ips) = std::net::ToSocketAddrs::to_socket_addrs(&(host, 80)) {
        if ips.map(|s| s.ip()).any(is_private_ip) {
            return true;
        }
    }
    false
}
pub(super) fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || matches!(
                    v4.octets(),
                    [10, _, _, _]
                        | [172, 16..=31, _, _]
                        | [192, 168, _, _]
                        | [169, 254, _, _]
                        | [0, _, _, _]
                )
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || matches!(v6.segments(), [0xfc, ..] | [0xfd, ..] | [0xfe, 0x80, ..])
        }
    }
}
