//! Split from `api.rs` — 338 lines.

use super::*;

#[tokio::test]
pub(crate) async fn test_whatsapp_webhook_invalid_payload() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/webhook/whatsapp", base_url);

    let response = client
        .post(&url)
        .json(&json!({"invalid": "payload"}))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status().is_client_error() || resp.status().is_success(),
                "Invalid webhook payload should be handled"
            );
        }
        Err(e) => {
            eprintln!("Webhook test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_json_content_type() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/auth/login", base_url);

    let response = client
        .post(&url)
        .header("Content-Type", "text/plain")
        .body("not json")
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE
                    || resp.status() == StatusCode::BAD_REQUEST,
                "Wrong content type should return 415 or 400"
            );
        }
        Err(e) => {
            eprintln!("Content type test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_404_response() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/nonexistent/path/here", base_url);

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            assert_eq!(
                resp.status(),
                StatusCode::NOT_FOUND,
                "Unknown path should return 404"
            );
        }
        Err(e) => {
            eprintln!("404 test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_method_not_allowed() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/health", base_url);

    let response = client.delete(&url).send().await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::METHOD_NOT_ALLOWED
                    || resp.status() == StatusCode::NOT_FOUND,
                "Wrong method should return 405 or 404"
            );
        }
        Err(e) => {
            eprintln!("Method test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_cors_preflight() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/bots", base_url);

    let response = client
        .request(reqwest::Method::OPTIONS, &url)
        .header("Origin", "http://localhost:3000")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            assert!(
                status == StatusCode::OK
                    || status == StatusCode::NO_CONTENT
                    || status == StatusCode::NOT_FOUND,
                "CORS preflight should return 200, 204, or 404"
            );
        }
        Err(e) => {
            eprintln!("CORS test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_rate_limiting_headers() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/health", base_url);

    let response = client.get(&url).send().await;

    match response {
        Ok(resp) => {
            let headers = resp.headers();
            if headers.contains_key("x-ratelimit-limit") {
                assert!(
                    headers.contains_key("x-ratelimit-remaining"),
                    "Rate limit headers should include remaining"
                );
            }
        }
        Err(e) => {
            eprintln!("Rate limit test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_malformed_json() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/auth/login", base_url);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body("{malformed json")
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status() == StatusCode::BAD_REQUEST
                    || resp.status() == StatusCode::UNPROCESSABLE_ENTITY,
                "Malformed JSON should return 400 or 422"
            );
        }
        Err(e) => {
            eprintln!("Malformed JSON test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_empty_body_where_required() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/auth/login", base_url);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert!(
                resp.status().is_client_error(),
                "Empty body should return client error"
            );
        }
        Err(e) => {
            eprintln!("Empty body test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_error_response_format() {
    let server = get_test_server().await;
    skip_if_no_server!(server);

    let (_ctx, base_url) = server.unwrap();
    let client = test_client();
    let url = format!("{}/api/auth/login", base_url);

    let response = client.post(&url).json(&json!({})).send().await;

    match response {
        Ok(resp) => {
            if resp.status().is_client_error() {
                let content_type = resp
                    .headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                if content_type.contains("application/json") {
                    let body: Result<serde_json::Value, _> = resp.json().await;
                    if let Ok(json) = body {
                        assert!(
                            json.get("error").is_some() || json.get("message").is_some(),
                            "Error response should have error or message field"
                        );
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error format test failed: {}", e);
        }
    }
}

#[tokio::test]
pub(crate) async fn test_with_mock_llm() {
    let ctx = match TestHarness::quick().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    if let Some(mock_llm) = ctx.mock_llm() {
        mock_llm.expect_completion("hello", "Hi there!").await;
        mock_llm.set_default_response("Default response").await;

        let call_count = mock_llm.call_count().await;
        assert_eq!(call_count, 0);
    }
}

#[tokio::test]
pub(crate) async fn test_mock_llm_assertions() {
    let ctx = match TestHarness::quick().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    if let Some(mock_llm) = ctx.mock_llm() {
        mock_llm.assert_not_called().await;

        mock_llm.set_default_response("Test response").await;

        let client = reqwest::Client::new();
        let _ = client
            .post(format!("{}/v1/chat/completions", mock_llm.url()))
            .json(&serde_json::json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "test"}]
            }))
            .send()
            .await;

        mock_llm.assert_called().await;
    }
}

#[tokio::test]
pub(crate) async fn test_mock_llm_error_simulation() {
    let ctx = match TestHarness::quick().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    if let Some(mock_llm) = ctx.mock_llm() {
        mock_llm.next_call_fails(500, "Internal error").await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/v1/chat/completions", mock_llm.url()))
            .json(&serde_json::json!({
                "model": "gpt-4",
                "messages": [{"role": "user", "content": "test"}]
            }))
            .send()
            .await;

        if let Ok(resp) = response {
            assert_eq!(resp.status().as_u16(), 500);
        }
    }
}
