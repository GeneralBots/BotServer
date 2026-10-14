use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[cfg(feature = "embed-ui")]
use crate::ui_server::constants::Assets;
use crate::ui_server::constants::get_ui_root;

fn get_cloud_url() -> String {
    std::env::var("CLOUD_URL").unwrap_or_else(|_| "http://localhost:4000".to_string())
}

async fn serve_login_file(file_path: std::path::PathBuf) -> Response {
    let injection = format!(
        r#"<script>window.GB_CLOUD_URL = "{}";</script>"#,
        get_cloud_url()
    );

    // Login pages reference shared cloud assets (e.g. /js/cloud-auth.js from
    // cloud/js/) — the login UI has no copy of them. Fall back to the cloud
    // tree per the "login serves CSS/JS/images from cloud via proxy" design;
    // otherwise the 404 HTML body reaches the browser as the script response
    // and strict MIME checking blocks it ("MIME type ('text/html') is not
    // executable").
    let cloud_fallback = {
        let ui_root = get_ui_root();
        file_path
            .strip_prefix(&ui_root)
            .ok()
            .and_then(|rel| rel.to_str())
            .and_then(|rel| rel.strip_prefix("login/"))
            .map(|rel| ui_root.join("cloud").join(rel))
    };

    #[cfg(feature = "embed-ui")]
    {
        let ui_root = get_ui_root();
        let relative = file_path.strip_prefix(&ui_root).unwrap_or(&file_path);
        let asset_path = relative.display().to_string().replace('\\', "/");
        let embedded = Assets::get(&asset_path)
            .or_else(|| {
                cloud_fallback
                    .as_ref()
                    .and_then(|p| p.strip_prefix(&ui_root).ok())
                    .and_then(|rel| rel.to_str())
                    .map(|rel| rel.replace('\\', "/"))
                    .and_then(|rel| Assets::get(&rel))
            });
        if let Some(content) = embedded {
            let mime = mime_guess::from_path(&asset_path).first_or_octet_stream();
            let data = if mime.as_ref() == "text/html" {
                inject_script_into_html(&content.data, &injection).into()
            } else {
                content.data
            };
            return ([(axum::http::header::CONTENT_TYPE, mime.as_ref())], data).into_response();
        }
        return StatusCode::NOT_FOUND.into_response();
    }

    #[cfg(not(feature = "embed-ui"))]
    {
        match tokio::fs::read(&file_path).await {
            Ok(bytes) => {
                let mime = mime_guess::from_path(&file_path).first_or_octet_stream();
                let data = if mime.as_ref() == "text/html" {
                    
                    inject_script_into_html(&bytes, &injection)
                } else {
                    bytes
                };
                ([(axum::http::header::CONTENT_TYPE, mime.as_ref())], data).into_response()
            }
            Err(_) => match cloud_fallback {
                Some(fallback_path) => match tokio::fs::read(&fallback_path).await {
                    Ok(bytes) => {
                        let mime =
                            mime_guess::from_path(&fallback_path).first_or_octet_stream();
                        ([(axum::http::header::CONTENT_TYPE, mime.as_ref())], bytes)
                            .into_response()
                    }
                    Err(_) => StatusCode::NOT_FOUND.into_response(),
                },
                None => StatusCode::NOT_FOUND.into_response(),
            },
        }
    }
}

fn inject_script_into_html(bytes: &[u8], script: &str) -> Vec<u8> {
    if let Ok(content) = std::str::from_utf8(bytes) {
        if let Some(head_end) = content.find("</head>") {
            let mut new_content = String::with_capacity(content.len() + script.len());
            new_content.push_str(&content[..head_end]);
            new_content.push_str(script);
            new_content.push_str(&content[head_end..]);
            return new_content.into_bytes();
        }
    }
    bytes.to_vec()
}

pub async fn serve_login_index() -> Response {
    let login_root = get_ui_root().join("login");
    serve_login_file(login_root.join("index.html")).await
}

pub async fn serve_login_signup() -> Response {
    let login_root = get_ui_root().join("login");
    serve_login_file(login_root.join("signup.html")).await
}

pub async fn serve_login_js(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let login_root = get_ui_root().join("login");
    serve_login_file(login_root.join("js").join(path.as_str())).await
}

pub async fn serve_login_images(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let login_root = get_ui_root().join("login");
    serve_login_file(login_root.join("images").join(path.as_str())).await
}

/// `/cloud/*path` on the login server — serves shared cloud assets (js,
/// images, css) directly from the cloud UI tree. Deployed login pages
/// reference `/cloud/js/cloud-auth.js` and `/cloud/images/logo.svg`; without
/// this route the request fell through to the index fallback and the browser
/// refused the HTML body as a script (strict MIME checking).
pub async fn serve_login_cloud(
    axum::extract::Path(path): axum::extract::Path<String>,
) -> Response {
    let cloud_root = get_ui_root().join("cloud");
    serve_login_file(cloud_root.join(path.as_str())).await
}
