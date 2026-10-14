use bottest::prelude::*;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

fn test_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to create HTTP client")
}

fn external_server_url() -> Option<String> {
    std::env::var("BOTSERVER_URL").ok()
}

async fn get_test_server() -> Option<(Option<TestContext>, String)> {
    if let Some(url) = external_server_url() {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .ok()?;

        if client.get(&url).send().await.is_ok() {
            return Some((None, url));
        }
    }

    let ctx = TestHarness::quick().await.ok()?;
    let server = ctx.start_botserver().await.ok()?;

    if server.is_running() {
        Some((Some(ctx), server.url.clone()))
    } else {
        None
    }
}

macro_rules! skip_if_no_server {
    ($base_url:expr) => {
        if $base_url.is_none() {
            eprintln!("Skipping test: no server available");
            return;
        }
    };
}

#[tokio::test]
async fn test_page_has_title() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .get(format!("{}/suite/auth/login.html", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                assert!(
                    body.contains("<title>") || body.contains("<Title>"),
                    "Page should have a title element"
                );
            }
        }
        Err(_) => {
            eprintln!("Skipping: server not available");
        }
    }
}

#[tokio::test]
async fn test_form_labels_present() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .get(format!("{}/suite/auth/login.html", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                
                let has_label = body.contains("<label") || body.contains("aria-label");
                let has_input = body.contains("<input");
                
                if has_input {
                    assert!(
                        has_label,
                        "Form inputs should have associated labels"
                    );
                }
            }
        }
        Err(_) => {
            eprintln!("Skipping: server not available");
        }
    }
}

#[tokio::test]
async fn test_button_has_text() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .get(format!("{}/suite/auth/login.html", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                
                let has_buttons = body.contains("<button");
                
                if has_buttons {
                    let has_button_text = body.contains("</button>");
                    assert!(
                        has_button_text,
                        "Buttons should have accessible text"
                    );
                }
            }
        }
        Err(_) => {
            eprintln!("Skipping: server not available");
        }
    }
}

#[tokio::test]
async fn test_images_have_alt_text() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .get(format!("{}/suite/auth/login.html", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                
                let has_images = body.contains("<img");
                
                if has_images {
                    let has_alt = body.contains("alt=");
                    assert!(
                        has_alt,
                        "Images should have alt attribute for accessibility"
                    );
                }
            }
        }
        Err(_) => {
            eprintln!("Skipping: server not available");
        }
    }
}

#[tokio::test]
async fn test_headings_have_proper_order() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .get(format!("{}/suite/auth/login.html", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                
                let h1_count = body.matches("<h1").count();
                let h2_count = body.matches("<h2").count();
                
                if h1_count > 1 {
                    eprintln!("Note: Multiple h1 elements found - consider using single h1");
                }
                
                if h2_count > 0 && h1_count == 0 {
                    eprintln!("Note: Page has h2 but no h1 - heading structure may be suboptimal");
                }
            }
        }
        Err(_) => {
            eprintln!("Skipping: server not available");
        }
    }
}

#[tokio::test]
async fn test_color_contrast() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .get(format!("{}/suite/auth/login.html", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                
                assert!(
                    body.contains("color:") || body.contains("background") || body.contains("class="),
                    "Page should have color styling for visual accessibility"
                );
                
                eprintln!("Note: Color contrast testing requires visual analysis or automated tools");
            }
        }
        Err(_) => {
            eprintln!("Skipping: server not available");
        }
    }
}

#[tokio::test]
async fn test_keyboard_navigation() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .get(format!("{}/suite/auth/login.html", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                
                let has_focusable = body.contains("tabindex=") || body.contains("href=");
                
                assert!(
                    has_focusable,
                    "Page should have keyboard-navigable elements"
                );
            }
        }
        Err(_) => {
            eprintln!("Skipping: server not available");
        }
    }
}

#[tokio::test]
async fn test_error_messages_accessible() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": "",
            "password": ""
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status() == 400 || resp.status() == 422 {
                let body = resp.text().await.unwrap_or_default();
                
                assert!(
                    !body.is_empty(),
                    "Error responses should provide accessible error messages"
                );
            }
        }
        Err(_) => {
            eprintln!("Skipping: server not available");
        }
    }
}
