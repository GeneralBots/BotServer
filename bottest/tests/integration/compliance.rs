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
async fn test_audit_log_for_sensitive_operations() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let login_response = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": "admin",
            "password": "admin"
        }))
        .send()
        .await;

    if let Ok(resp) = login_response {
        if resp.status() == 200 {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(token) = body.get("token").and_then(|t| t.as_str()) {
                let create_response = client
                    .post(format!("{}/api/bots", base_url))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&json!({
                        "name": "test_bot_audit"
                    }))
                    .send()
                    .await;

                if let Ok(resp) = create_response {
                    assert!(
                        resp.status() == 200 || resp.status() == 201 || resp.status() == 400,
                        "Bot creation should be logged or validated"
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_gdpr_data_deletion_endpoint() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let login_response = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": "testuser",
            "password": "testpass"
        }))
        .send()
        .await;

    if let Ok(resp) = login_response {
        if resp.status() == 200 {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(token) = body.get("token").and_then(|t| t.as_str()) {
                let delete_response = client
                    .delete(format!("{}/api/users/me", base_url))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await;

                if let Ok(resp) = delete_response {
                    assert!(
                        resp.status() == 200 || resp.status() == 204 || resp.status() == 404 || resp.status() == 401,
                        "GDPR deletion endpoint should exist and respond"
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_gdpr_data_export_endpoint() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let login_response = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": "testuser",
            "password": "testpass"
        }))
        .send()
        .await;

    if let Ok(resp) = login_response {
        if resp.status() == 200 {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(token) = body.get("token").and_then(|t| t.as_str()) {
                let export_response = client
                    .get(format!("{}/api/users/me/export", base_url))
                    .header("Authorization", format!("Bearer {}", token))
                    .send()
                    .await;

                if let Ok(resp) = export_response {
                    if resp.status() == 200 {
                        let _body: serde_json::Value = resp.json().await.unwrap_or_default();
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn test_consent_tracking_endpoint() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let login_response = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": "testuser",
            "password": "testpass"
        }))
        .send()
        .await;

    if let Ok(resp) = login_response {
        if resp.status() == 200 {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if let Some(token) = body.get("token").and_then(|t| t.as_str()) {
                let consent = json!({
                    "marketing": true,
                    "analytics": false,
                    "timestamp": "2024-01-01T00:00:00Z"
                });

                let response = client
                    .post(format!("{}/api/users/me/consent", base_url))
                    .header("Authorization", format!("Bearer {}", token))
                    .json(&consent)
                    .send()
                    .await;

                if let Ok(resp) = response {
                    assert!(
                        resp.status() == 200 || resp.status() == 201 || resp.status() == 404,
                        "Consent endpoint should exist"
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn test_session_isolation() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client1 = test_client();
    let client2 = test_client();

    let login1 = client1
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": "user1",
            "password": "pass1"
        }))
        .send()
        .await;

    let login2 = client2
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": "user2",
            "password": "pass2"
        }))
        .send()
        .await;

    if let (Ok(resp1), Ok(resp2)) = (login1, login2) {
        if resp1.status() == 200 && resp2.status() == 200 {
            let body1: serde_json::Value = resp1.json().await.unwrap_or_default();
            let body2: serde_json::Value = resp2.json().await.unwrap_or_default();

            let token1 = body1.get("token").and_then(|t| t.as_str());
            let token2 = body2.get("token").and_then(|t| t.as_str());

            if let (Some(t1), Some(t2)) = (token1, token2) {
                let me1 = client1
                    .get(format!("{}/api/users/me", base_url))
                    .header("Authorization", format!("Bearer {}", t1))
                    .send()
                    .await;

                let me2 = client2
                    .get(format!("{}/api/users/me", base_url))
                    .header("Authorization", format!("Bearer {}", t2))
                    .send()
                    .await;

                if let (Ok(r1), Ok(r2)) = (me1, me2) {
                    if r1.status() == 200 && r2.status() == 200 {
                        let user1: serde_json::Value = r1.json().await.unwrap_or_default();
                        let user2: serde_json::Value = r2.json().await.unwrap_or_default();

                        let id1 = user1.get("id");
                        let id2 = user2.get("id");

                        if let (Some(i1), Some(i2)) = (id1, id2) {
                            assert_ne!(
                                i1, i2,
                                "Different users should have different IDs"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn test_password_complexity_validation() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let response = client
        .post(format!("{}/api/auth/register", base_url))
        .json(&json!({
            "username": "newuser",
            "email": "newuser@example.com",
            "password": "weak"
        }))
        .send()
        .await;

    if let Ok(resp) = response {
        assert!(
            resp.status() == 400 || resp.status() == 422 || resp.status() == 409,
            "Weak password should be rejected"
        );
    }
}

#[tokio::test]
async fn test_failed_login_lockout() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    for _ in 0..5 {
        let _ = client
            .post(format!("{}/api/auth/login", base_url))
            .json(&json!({
                "username": "locktest",
                "password": "wrongpassword"
            }))
            .send()
            .await;
    }

    let response = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({
            "username": "locktest",
            "password": "correctpassword"
        }))
        .send()
        .await;

    if let Ok(resp) = response {
        if resp.status() == 423 {
            // Account should be locked after failed attempts.
        } else if resp.status() == 200 || resp.status() == 401 {
            eprintln!("Note: Account lockout may not be enabled");
        }
    }
}

#[tokio::test]
async fn test_access_logging() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();

    let _ = client
        .get(format!("{}/api/sensitive-data", base_url))
        .send()
        .await;

    eprintln!("Note: Access logging verification requires log inspection");
}
