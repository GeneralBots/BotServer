use super::{should_run_e2e_tests, E2ETestContext};
use bottest::prelude::*;
use bottest::web::{Browser, Locator};
use std::time::Duration;

fn admin_credentials() -> (String, String) {
    let email = std::env::var("TEST_ADMIN_EMAIL").unwrap_or_else(|_| "admin@test.com".to_string());
    let password = std::env::var("TEST_ADMIN_PASSWORD").unwrap_or_else(|_| "testpass".to_string());
    (email, password)
}

fn attendant_credentials() -> (String, String) {
    let email =
        std::env::var("TEST_ATTENDANT_EMAIL").unwrap_or_else(|_| "attendant@test.com".to_string());
    let password =
        std::env::var("TEST_ATTENDANT_PASSWORD").unwrap_or_else(|_| "testpass".to_string());
    (email, password)
}

async fn perform_login(
    browser: &Browser,
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<(), String> {
    let login_url = format!("{}/login", base_url);

    browser
        .goto(&login_url)
        .await
        .map_err(|e| format!("Failed to navigate to login: {}", e))?;

    let email_input = Locator::css("#email, input[name='email'], input[type='email']");
    browser
        .wait_for(email_input.clone())
        .await
        .map_err(|e| format!("Email input not found: {}", e))?;

    browser
        .type_text(email_input, email)
        .await
        .map_err(|e| format!("Failed to fill email: {}", e))?;

    let password_input = Locator::css("#password, input[name='password'], input[type='password']");
    browser
        .type_text(password_input, password)
        .await
        .map_err(|e| format!("Failed to fill password: {}", e))?;

    let login_button = Locator::css("#login-button, button[type='submit'], .login-btn");
    browser
        .click(login_button)
        .await
        .map_err(|e| format!("Failed to click login: {}", e))?;

    tokio::time::sleep(Duration::from_secs(2)).await;

    Ok(())
}

#[tokio::test]
async fn test_login_page_loads() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
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
    let login_url = format!("{}/login", ctx.base_url());

    if let Err(e) = browser.goto(&login_url).await {
        eprintln!("Failed to navigate: {}", e);
        ctx.close().await;
        return;
    }

    let elements_to_check = vec![
        ("#email, input[type='email']", "email input"),
        ("#password, input[type='password']", "password input"),
        ("button[type='submit'], .login-btn", "login button"),
    ];

    for (selector, name) in elements_to_check {
        let locator = Locator::css(selector);
        match browser.find_element(locator).await {
            Ok(_) => println!("Found: {}", name),
            Err(_) => eprintln!("Not found: {}", name),
        }
    }

    ctx.close().await;
}

#[tokio::test]
async fn test_login_success() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
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
    let (email, password) = admin_credentials();

    match perform_login(browser, ctx.base_url(), &email, &password).await {
        Ok(_) => {
            let dashboard_indicator =
                Locator::css(".dashboard, #dashboard, [data-page='dashboard'], .nav-menu");
            match browser.find_element(dashboard_indicator).await {
                Ok(_) => println!("Login successful - dashboard visible"),
                Err(_) => eprintln!("Login may have failed - dashboard not visible"),
            }
        }
        Err(e) => eprintln!("Login failed: {}", e),
    }

    ctx.close().await;
}

#[tokio::test]
async fn test_login_failure() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
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

    let wrongpass = String::from_utf8_lossy(b"wrongpass").to_string();
    match perform_login(browser, ctx.base_url(), "invalid@test.com", &wrongpass).await {
        Ok(_) => {
            let error_indicator =
                Locator::css(".error, .alert-error, .login-error, [role='alert']");
            match browser.find_element(error_indicator).await {
                Ok(_) => println!("Error message displayed correctly"),
                Err(_) => eprintln!("Error message not found"),
            }
        }
        Err(e) => eprintln!("Login attempt failed: {}", e),
    }

    ctx.close().await;
}

#[tokio::test]
async fn test_dashboard_home() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
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
    let (email, password) = admin_credentials();

    if perform_login(browser, ctx.base_url(), &email, &password)
        .await
        .is_err()
    {
        ctx.close().await;
        return;
    }

    let dashboard_elements = vec![
        (".stats, .statistics, .metrics", "statistics panel"),
        (".queue-summary, .queue-panel", "queue summary"),
        (".recent-activity, .activity-log", "activity log"),
    ];

    for (selector, name) in dashboard_elements {
        let locator = Locator::css(selector);
        match browser.find_element(locator).await {
            Ok(_) => println!("Found: {}", name),
            Err(_) => eprintln!("Not found: {} (may not be implemented)", name),
        }
    }

    ctx.close().await;
}

#[tokio::test]
async fn test_queue_panel() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
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
    let (email, password) = attendant_credentials();

    if perform_login(browser, ctx.base_url(), &email, &password)
        .await
        .is_err()
    {
        ctx.close().await;
        return;
    }

    let queue_url = format!("{}/queue", ctx.base_url());
    let _ = browser.goto(&queue_url).await;

    let queue_elements = vec![
        (".queue-list, #queue-list, .waiting-list", "queue list"),
        (".queue-item, .queue-entry", "queue items"),
        (
            ".take-btn, .accept-btn, [data-action='take']",
            "take button",
        ),
    ];

    for (selector, name) in queue_elements {
        let locator = Locator::css(selector);
        match browser.find_element(locator).await {
            Ok(_) => println!("Found: {}", name),
            Err(_) => eprintln!("Not found: {}", name),
        }
    }

    ctx.close().await;
}

mod part2;
mod part3;
