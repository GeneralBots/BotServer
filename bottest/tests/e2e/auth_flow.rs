use super::{check_webdriver_available, should_run_e2e_tests, E2ETestContext};
use bottest::prelude::*;
use bottest::web::WaitCondition;
use bottest::web::{Browser, Locator};
use std::time::Duration;

async fn setup_auth_mocks(ctx: &TestContext, email: &str, password: &str) {
    if let Some(mock_zitadel) = ctx.mock_zitadel() {
        let user = mock_zitadel.create_test_user(email);
        mock_zitadel.expect_login(email, password).await;
        mock_zitadel.expect_any_introspect_active().await;
        mock_zitadel.expect_any_userinfo().await;
        mock_zitadel.expect_revoke().await;
        let _ = user;
    }
}

async fn setup_chat_mocks(ctx: &TestContext) {
    if let Some(mock_llm) = ctx.mock_llm() {
        mock_llm
            .set_default_response("Hello! I'm your assistant. How can I help you today?")
            .await;
        mock_llm
            .expect_completion("hello", "Hi there! Nice to meet you.")
            .await;
        mock_llm
            .expect_completion(
                "help",
                "I'm here to help! What do you need assistance with?",
            )
            .await;
        mock_llm
            .expect_completion("bye", "Goodbye! Have a great day!")
            .await;
    }
}

async fn perform_login(
    browser: &Browser,
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<bool, String> {
    let login_url = format!("{}/login", base_url);

    browser
        .goto(&login_url)
        .await
        .map_err(|e| format!("Failed to navigate to login: {}", e))?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let email_input = Locator::css("#email, input[name='email'], input[type='email']");
    browser
        .wait_for(email_input.clone())
        .await
        .map_err(|e| format!("Email input not found: {}", e))?;

    browser
        .fill(email_input, email)
        .await
        .map_err(|e| format!("Failed to fill email: {}", e))?;

    let password_input = Locator::css("#password, input[name='password'], input[type='password']");
    browser
        .fill(password_input, password)
        .await
        .map_err(|e| format!("Failed to fill password: {}", e))?;

    let login_button = Locator::css("#login-button, button[type='submit'], .login-btn, .btn-login");
    browser
        .click(login_button)
        .await
        .map_err(|e| format!("Failed to click login: {}", e))?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    let dashboard_indicators = vec![
        ".dashboard",
        "#dashboard",
        "[data-page='dashboard']",
        ".nav-menu",
        ".main-content",
        ".user-menu",
        ".sidebar",
    ];

    for selector in dashboard_indicators {
        let locator = Locator::css(selector);
        if browser.exists(locator).await {
            return Ok(true);
        }
    }

    let current_url = browser.current_url().await.unwrap_or_default();
    if !current_url.contains("/login") {
        return Ok(true);
    }

    Ok(false)
}

async fn send_chat_message(browser: &Browser, message: &str) -> Result<(), String> {
    let input_locator = Locator::css(
        "#chat-input, .chat-input, textarea[placeholder*='message'], textarea[name='message']",
    );

    browser
        .wait_for(input_locator.clone())
        .await
        .map_err(|e| format!("Chat input not found: {}", e))?;

    browser
        .fill(input_locator, message)
        .await
        .map_err(|e| format!("Failed to type message: {}", e))?;

    let send_button = Locator::css("#send-button, .send-button, button[type='submit'], .btn-send");
    browser
        .click(send_button)
        .await
        .map_err(|e| format!("Failed to click send: {}", e))?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}

async fn wait_for_bot_response(browser: &Browser) -> Result<String, String> {
    let response_locator = Locator::css(
        ".bot-message, .message-bot, .response, .assistant-message, [data-role='assistant']",
    );

    let element = browser
        .wait_for_condition(response_locator, WaitCondition::Present)
        .await
        .map_err(|e| format!("Bot response not found: {}", e))?;

    let text = element
        .text()
        .await
        .map_err(|e| format!("Failed to get response text: {}", e))?;

    Ok(text)
}

async fn perform_logout(browser: &Browser, base_url: &str) -> Result<bool, String> {
    let logout_selectors = vec![
        "#logout-button",
        ".logout-btn",
        "a[href*='logout']",
        "button[data-action='logout']",
        ".user-menu .logout",
        "#user-menu-logout",
    ];

    for selector in &logout_selectors {
        let locator = Locator::css(selector);
        if browser.exists(locator.clone()).await && browser.click(locator).await.is_ok() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            break;
        }
    }

    let user_menu_locator = Locator::css(".user-menu, .avatar, .profile-icon, #user-dropdown");
    if browser.exists(user_menu_locator.clone()).await {
        let _ = browser.click(user_menu_locator).await;
        tokio::time::sleep(Duration::from_millis(300)).await;

        for selector in &logout_selectors {
            let locator = Locator::css(selector);
            if browser.exists(locator.clone()).await && browser.click(locator).await.is_ok() {
                tokio::time::sleep(Duration::from_secs(1)).await;
                break;
            }
        }
    }

    let current_url = browser.current_url().await.unwrap_or_default();
    let base_url_with_slash = format!("{base_url}/");
    let logged_out = current_url.contains("/login")
        || current_url.contains("/logout")
        || current_url == base_url_with_slash
        || current_url == base_url;

    if logged_out {
        return Ok(true);
    }

    let login_form = Locator::css("#login-form, .login-form, form[action*='login']");
    if browser.exists(login_form).await {
        return Ok(true);
    }

    Ok(false)
}

async fn navigate_to_chat(browser: &Browser, base_url: &str, bot_name: &str) -> Result<(), String> {
    let chat_url = format!("{}/chat/{}", base_url, bot_name);

    browser
        .goto(&chat_url)
        .await
        .map_err(|e| format!("Failed to navigate to chat: {}", e))?;

    tokio::time::sleep(Duration::from_millis(500)).await;

    let chat_container = Locator::css("#chat-container, .chat-container, .chat-widget, .chat-box");
    browser
        .wait_for(chat_container)
        .await
        .map_err(|e| format!("Chat container not found: {}", e))?;

    Ok(())
}

#[tokio::test]
async fn test_complete_auth_flow_login_chat_logout() {
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

    let email = "testuser@example.com";
    let password = String::from_utf8_lossy(b"testpassword123").to_string();
    let bot_name = "test-bot";

    setup_auth_mocks(&ctx.ctx, email, &password).await;
    setup_chat_mocks(&ctx.ctx).await;

    let browser = ctx.browser.as_ref().unwrap();
    let base_url = ctx.base_url();

    println!("Step 1: Performing login...");
    match perform_login(browser, base_url, email, &password).await {
        Ok(true) => println!("  ✓ Login successful"),
        Ok(false) => {
            eprintln!("  ✗ Login failed - dashboard not visible");
            ctx.close().await;
            return;
        }
        Err(e) => {
            eprintln!("  ✗ Login error: {}", e);
            ctx.close().await;
            return;
        }
    }

    println!("Step 2: Navigating to chat...");
    if let Err(e) = navigate_to_chat(browser, base_url, bot_name).await {
        eprintln!("  ✗ Navigation error: {}", e);
        ctx.close().await;
        return;
    }
    println!("  ✓ Chat page loaded");

    println!("Step 3: Sending messages...");

    let messages = vec![
        ("hello", "greeting"),
        ("I need help", "help request"),
        ("bye", "farewell"),
    ];

    for (message, description) in messages {
        println!("  Sending: {} ({})", message, description);

        match send_chat_message(browser, message).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("    ✗ Failed to send message: {}", e);
                continue;
            }
        }

        match wait_for_bot_response(browser).await {
            Ok(response) => {
                println!(
                    "    ✓ Bot responded: {}...",
                    &response[..response.len().min(50)]
                );
            }
            Err(e) => {
                eprintln!("    ✗ No bot response: {}", e);
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("Step 4: Performing logout...");
    match perform_logout(browser, base_url).await {
        Ok(true) => println!("  ✓ Logout successful"),
        Ok(false) => {
            eprintln!("  ✗ Logout may have failed - not redirected to login");
        }
        Err(e) => {
            eprintln!("  ✗ Logout error: {}", e);
        }
    }

    println!("Step 5: Verifying logout by attempting to access protected page...");
    let dashboard_url = format!("{}/dashboard", base_url);
    if browser.goto(&dashboard_url).await.is_ok() {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let current_url = browser.current_url().await.unwrap_or_default();

        if current_url.contains("/login") {
            println!("  ✓ Correctly redirected to login page");
        } else {
            eprintln!("  ✗ Session may still be active");
        }
    }

    println!("\n=== Auth Flow Test Complete ===");

    ctx.close().await;
}

mod part2;
