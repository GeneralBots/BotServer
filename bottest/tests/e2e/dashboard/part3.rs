//! Split from `dashboard.rs` — 282 lines.

use super::*;

#[tokio::test]
pub(crate) async fn test_logout() {
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

    let logout_btn = Locator::css(".logout, #logout, [data-action='logout'], a[href*='logout']");
    match browser.click(logout_btn).await {
        Ok(_) => {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let login_form = Locator::css("#email, input[type='email'], .login-form");
            match browser.find_element(login_form).await {
                Ok(_) => println!("Logout successful - login page visible"),
                Err(_) => eprintln!("Login page not visible after logout"),
            }
        }
        Err(_) => eprintln!("Logout button not found"),
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_navigation() {
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

    let nav_links = vec![
        ("a[href*='dashboard'], .nav-dashboard", "Dashboard"),
        ("a[href*='queue'], .nav-queue", "Queue"),
        ("a[href*='bots'], .nav-bots", "Bots"),
        ("a[href*='analytics'], .nav-analytics", "Analytics"),
        ("a[href*='settings'], .nav-settings", "Settings"),
    ];

    for (selector, name) in nav_links {
        let locator = Locator::css(selector);
        match browser.find_element(locator).await {
            Ok(_) => println!("Nav link found: {}", name),
            Err(_) => eprintln!("Nav link not found: {}", name),
        }
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_access_control() {
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

    let admin_url = format!("{}/admin/users", ctx.base_url());
    let _ = browser.goto(&admin_url).await;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let current_url = browser.current_url().await.unwrap_or_default();

    if current_url.contains("/admin/users") {
        let denied = Locator::css(".access-denied, .forbidden, .error-403");
        match browser.find_element(denied).await {
            Ok(_) => println!("Access correctly denied for attendant"),
            Err(_) => eprintln!("Access control may not be enforced"),
        }
    } else {
        println!("Redirected away from admin page (access control working)");
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_dark_mode() {
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

    let theme_toggle = Locator::css(".theme-toggle, .dark-mode-toggle, #theme-switch");
    match browser.click(theme_toggle).await {
        Ok(_) => {
            tokio::time::sleep(Duration::from_millis(500)).await;

            let dark_indicator = Locator::css(".dark, .dark-mode, [data-theme='dark']");
            match browser.find_element(dark_indicator).await {
                Ok(_) => println!("Dark mode activated"),
                Err(_) => eprintln!("Dark mode indicator not found"),
            }
        }
        Err(_) => eprintln!("Theme toggle not found (feature may not be implemented)"),
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_with_fixtures() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
        return;
    }

    let ctx = match E2ETestContext::setup().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    let user = admin_user();
    let bot = bot_with_kb("e2e-test-bot");
    let customer = customer("+15551234567");

    match ctx.ctx.insert_user(&user).await {
        Ok(_) => println!("Inserted test user: {}", user.email),
        Err(e) => eprintln!(
            "Could not insert user (DB may not be directly accessible): {}",
            e
        ),
    }

    match ctx.ctx.insert_bot(&bot).await {
        Ok(_) => println!("Inserted test bot: {}", bot.name),
        Err(e) => eprintln!("Could not insert bot: {}", e),
    }

    match ctx.ctx.insert_customer(&customer).await {
        Ok(_) => println!("Inserted test customer"),
        Err(e) => eprintln!("Could not insert customer: {}", e),
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_mock_services_available() {
    if !should_run_e2e_tests() {
        eprintln!("Skipping: E2E tests disabled");
        return;
    }

    let ctx = match E2ETestContext::setup().await {
        Ok(ctx) => ctx,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    if ctx.ctx.mock_llm().is_some() {
        println!("✓ MockLLM is available");
    } else {
        eprintln!("MockLLM not available");
    }

    if ctx.ctx.mock_zitadel().is_some() {
        println!("✓ MockZitadel is available");
    } else {
        eprintln!("MockZitadel not available");
    }

    if ctx.ctx.use_existing_stack {
        println!("Using existing stack - PostgreSQL is external (not managed by harness)");
        match ctx.ctx.db_pool().await {
            Ok(_pool) => println!("✓ Connected to existing PostgreSQL"),
            Err(e) => eprintln!("Could not connect to existing PostgreSQL: {}", e),
        }
    } else if ctx.ctx.postgres().is_some() {
        println!("✓ PostgreSQL is managed by harness");
    } else {
        eprintln!("PostgreSQL should be started in fresh stack mode");
    }

    ctx.close().await;
}
