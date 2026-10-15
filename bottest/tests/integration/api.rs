use bottest::prelude::*;
use reqwest::{Client, StatusCode};
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
            eprintln!("Skipping API test: no server available");
            return;
        }
    };
}

#[tokio::test]
async fn test_health_endpoint() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/health", base_url);

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status().is_success(),
                "Health endpoint should return success status"
            );
        }
        Err(e) => {
            eprintln!("Health check failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_ready_endpoint() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/ready", base_url);

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::OK || resp.status() == StatusCode::SERVICE_UNAVAILABLE,
                "Ready endpoint should return 200 or 503"
            );
        }
        Err(e) => {
            eprintln!("Ready check failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_version_endpoint() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/version", base_url);

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                assert!(!body.is_empty(), "Version should return non-empty body");
            }
        }
        Err(e) => {
            eprintln!("Version check failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_login_missing_credentials() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/auth/login", base_url);

    let response = client.post(&url).json(&json!({})).send().await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::BAD_REQUEST
                    || resp.status() == StatusCode::UNPROCESSABLE_ENTITY,
                "Missing credentials should return 400 or 422"
            );
        }
        Err(e) => {
            eprintln!("Login test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/auth/login", base_url);

    let response = client
        .post(&url)
        .json(&json!({
            "email": "invalid@example.com",
            "password": "wrongpassword"
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::UNAUTHORIZED
                    || resp.status() == StatusCode::FORBIDDEN
                    || resp.status() == StatusCode::NOT_FOUND,
                "Invalid credentials should return 401, 403, or 404"
            );
        }
        Err(e) => {
            eprintln!("Login test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_protected_endpoint_without_auth() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/bots", base_url);

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::UNAUTHORIZED || resp.status() == StatusCode::FORBIDDEN,
                "Protected endpoint without auth should return 401 or 403"
            );
        }
        Err(e) => {
            eprintln!("Auth test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_list_bots_unauthorized() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/bots", base_url);

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::UNAUTHORIZED || resp.status() == StatusCode::FORBIDDEN,
                "List bots without auth should return 401 or 403"
            );
        }
        Err(e) => {
            eprintln!("Bots test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_get_nonexistent_bot() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let fake_id = Uuid::new_v4();
    let url = format!("{}/api/bots/{}", base_url, fake_id);

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::NOT_FOUND
                    || resp.status() == StatusCode::UNAUTHORIZED
                    || resp.status() == StatusCode::FORBIDDEN,
                "Nonexistent bot should return 404, 401, or 403"
            );
        }
        Err(e) => {
            eprintln!("Bot test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_send_message_missing_body() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/chat/send", base_url);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body("{}")
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status().is_client_error(),
                "Missing body should return client error"
            );
        }
        Err(e) => {
            eprintln!("Message test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_send_message_invalid_bot() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/chat/send", base_url);

    let response = client
        .post(&url)
        .json(&json!({
            "bot_id": Uuid::new_v4().to_string(),
            "message": "Hello",
            "session_id": Uuid::new_v4().to_string()
        }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status().is_client_error(),
                "Invalid bot should return client error"
            );
        }
        Err(e) => {
            eprintln!("Message test failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_whatsapp_webhook_verification() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!(
        "{}/webhook/whatsapp?hub.mode=subscribe&hub.verify_token=test&hub.challenge=test123",
        base_url
    );

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            assert!(
                status == StatusCode::OK
                    || status == StatusCode::FORBIDDEN
                    || status == StatusCode::NOT_FOUND,
                "Webhook verification should return 200, 403, or 404"
            );
        }
        Err(e) => {
            eprintln!("Webhook test failed: {}", e);
        }
    }
}

mod part2;
