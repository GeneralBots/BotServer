//! Split from `suite.rs` — 258 lines.

use super::*;

pub async fn serve_suite(
    State(state): State<AppState>,
    Query(params): Query<SuiteQueryParams>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    serve_suite_impl(&state, params.bot_name, headers).await
}

pub async fn serve_suite_impl(
    _state: &AppState,
    bot_name: Option<String>,
    _headers: axum::http::HeaderMap,
) -> Response {
    let is_auth = bot_name
        .as_ref()
        .map(|n| {
            n.ends_with(".html")
                || n == "login"
                || n == "register"
                || n == "forgot-password"
                || n == "reset-password"
        })
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
            let _ = &mut html;

            if let Some(head_end) = html.find("</head>") {
                let is_auth_page = bot_name
                    .as_ref()
                    .map(|n| {
                        n.ends_with(".html")
                            || n == "login"
                            || n == "register"
                            || n == "forgot-password"
                            || n == "reset-password"
                    })
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
                    let login_script =
                        format!(r#"<script>window.GB_LOGIN_URL = "{}";</script>"#, login_url);
                    html.insert_str(head_end + base_tag.len(), &login_script);

                    if let Some(name) = bot_name {
                        let ins_offset = head_end + base_tag.len() + login_script.len();
                        info!(
                            "serve_suite: Injecting bot_name '{}' into page with base href='{}'",
                            name, base_href
                        );
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

            (
                StatusCode::OK,
                [
                    ("content-type", "text/html; charset=utf-8"),
                    ("cache-control", "no-cache, no-store, must-revalidate"),
                    ("pragma", "no-cache"),
                    ("expires", "0"),
                ],
                Html(html),
            )
                .into_response()
        }
        Err(e) => {
            error!("Failed to load suite UI: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                Html("Failed to load suite interface".to_string()),
            )
                .into_response()
        }
    }
}
