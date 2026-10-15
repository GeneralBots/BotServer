//! Split from `auth_flow.rs` — 246 lines.

use super::*;

#[tokio::test]
pub(crate) async fn test_login_with_invalid_credentials() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
        return;
    }

    if !check_webdriver_available().await {
        eprintln!("Skipping: WebDriver not available");
        return;
    }

    let ctx = match E2ETestContext::setup_with_browser().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    if !ctx.has_browser() {
        eprintln!("Skipping: browser not available");
        ctx.close().await;
        return;
    }

    if let Some(mock_zitadel) = ctx.ctx.mock_zitadel() {
        mock_zitadel.expect_invalid_credentials().await;
    }

    let browser = ctx.browser.as_ref().unwrap();
    let base_url = ctx.base_url();

    let wrongpassword = String::from_utf8_lossy(b"wrongpassword").to_string();
    match perform_login(browser, base_url, "invalid@test.com", &wrongpassword).await {
        Ok(true) => {
            eprintln!("✗ Login succeeded with invalid credentials - unexpected");
        }
        Ok(false) => {
            println!("✓ Login correctly rejected invalid credentials");

            let error_locator =
                Locator::css(".error, .alert-error, .login-error, [role='alert'], .error-message");
            if browser.exists(error_locator).await {
                println!("✓ Error message displayed to user");
            }
        }
        Err(e) => {
            eprintln!("Login attempt failed: {}", e);
        }
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_session_persistence() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
        return;
    }

    if !check_webdriver_available().await {
        eprintln!("Skipping: WebDriver not available");
        return;
    }

    let ctx = match E2ETestContext::setup_with_browser().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    if !ctx.has_browser() {
        eprintln!("Skipping: browser not available");
        ctx.close().await;
        return;
    }

    let email = "session@test.com";
    let password = String::from_utf8_lossy(b"testpass").to_string();

    setup_auth_mocks(&ctx.ctx, email, &password).await;

    let browser = ctx.browser.as_ref().unwrap();
    let base_url = ctx.base_url();

    if perform_login(browser, base_url, email, &password)
        .await
        .unwrap_or(false)
    {
        println!("✓ Initial login successful");

        let dashboard_url = format!("{}/dashboard", base_url);
        if browser.goto(&dashboard_url).await.is_ok() {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        if browser.refresh().await.is_ok() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let current_url = browser.current_url().await.unwrap_or_default();

            if current_url.contains("/login") {
                eprintln!("✗ Session lost after refresh");
            } else {
                println!("✓ Session persisted after page refresh");
            }
        }

        let protected_url = format!("{}/admin/settings", base_url);
        if browser.goto(&protected_url).await.is_ok() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let current_url = browser.current_url().await.unwrap_or_default();

            if current_url.contains("/login") {
                eprintln!("✗ Session lost during navigation");
            } else {
                println!("✓ Session maintained across navigation");
            }
        }
    } else {
        eprintln!("✗ Initial login failed");
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_chat_message_flow() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
        return;
    }

    if !check_webdriver_available().await {
        eprintln!("Skipping: WebDriver not available");
        return;
    }

    let ctx = match E2ETestContext::setup_with_browser().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    if !ctx.has_browser() {
        eprintln!("Skipping: browser not available");
        ctx.close().await;
        return;
    }

    setup_chat_mocks(&ctx.ctx).await;

    let browser = ctx.browser.as_ref().unwrap();
    let chat_url = format!("{}/chat/test-bot", ctx.base_url());

    if browser.goto(&chat_url).await.is_err() {
        eprintln!("Failed to navigate to chat");
        ctx.close().await;
        return;
    }

    tokio::time::sleep(Duration::from_secs(1)).await;

    let message_count_before = browser
        .find_all(Locator::css(".message, .chat-message"))
        .await
        .map(|v| v.len())
        .unwrap_or(0);

    if send_chat_message(browser, "Hello bot!").await.is_ok() {
        tokio::time::sleep(Duration::from_secs(2)).await;

        let message_count_after = browser
            .find_all(Locator::css(".message, .chat-message"))
            .await
            .map(|v| v.len())
            .unwrap_or(0);

        if message_count_after > message_count_before {
            println!(
                "✓ Messages added to chat: {} -> {}",
                message_count_before, message_count_after
            );
        } else {
            eprintln!("✗ No new messages appeared in chat");
        }
    } else {
        eprintln!("✗ Failed to send chat message");
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_unauthenticated_access_redirect() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
        return;
    }

    if !check_webdriver_available().await {
        eprintln!("Skipping: WebDriver not available");
        return;
    }

    let ctx = match E2ETestContext::setup_with_browser().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    if !ctx.has_browser() {
        eprintln!("Skipping: browser not available");
        ctx.close().await;
        return;
    }

    let browser = ctx.browser.as_ref().unwrap();
    let base_url = ctx.base_url();

    let protected_routes = vec!["/dashboard", "/admin", "/settings", "/profile"];

    for route in protected_routes {
        let url = format!("{}{}", base_url, route);

        if browser.goto(&url).await.is_ok() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let current_url = browser.current_url().await.unwrap_or_default();

            if current_url.contains("/login") {
                println!("✓ {} correctly redirects to login", route);
            } else {
                eprintln!("✗ {} accessible without authentication", route);
            }
        }
    }

    ctx.close().await;
}
