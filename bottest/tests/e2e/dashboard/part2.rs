//! Split from `dashboard.rs` — 265 lines.

use super::*;

#[tokio::test]
pub(crate) async fn test_bot_management() {
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

    let bots_url = format!("{}/admin/bots", ctx.base_url());
    let _ = browser.goto(&bots_url).await;

    let bot_elements = vec![
        (".bot-list, #bot-list, .bots-table", "bot list"),
        (
            ".create-bot, .add-bot, [data-action='create']",
            "create button",
        ),
    ];

    for (selector, name) in bot_elements {
        let locator = Locator::css(selector);
        match browser.find_element(locator).await {
            Ok(_) => println!("Found: {}", name),
            Err(_) => eprintln!("Not found: {}", name),
        }
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_create_bot() {
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

    let create_url = format!("{}/admin/bots/new", ctx.base_url());
    let _ = browser.goto(&create_url).await;

    let name_input = Locator::css("#bot-name, input[name='name'], .bot-name-input");
    if browser.wait_for(name_input.clone()).await.is_ok() {
        let bot_name = format!("test-bot-{}", Uuid::new_v4());
        let _ = browser.type_text(name_input, &bot_name).await;

        let submit_btn = Locator::css("button[type='submit'], .save-btn, .create-btn");
        let _ = browser.click(submit_btn).await;

        tokio::time::sleep(Duration::from_secs(1)).await;

        let success_indicator = Locator::css(".success, .alert-success, .toast-success");
        match browser.find_element(success_indicator).await {
            Ok(_) => println!("Bot created successfully"),
            Err(_) => eprintln!("Success indicator not found"),
        }
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_knowledge_base() {
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

    let kb_url = format!("{}/admin/knowledge", ctx.base_url());
    let _ = browser.goto(&kb_url).await;

    let kb_elements = vec![
        (".document-list, .kb-documents", "document list"),
        (".upload-btn, .add-document", "upload button"),
        (".search-kb, .kb-search", "search input"),
    ];

    for (selector, name) in kb_elements {
        let locator = Locator::css(selector);
        match browser.find_element(locator).await {
            Ok(_) => println!("Found: {}", name),
            Err(_) => eprintln!("Not found: {}", name),
        }
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_analytics() {
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

    let analytics_url = format!("{}/admin/analytics", ctx.base_url());
    let _ = browser.goto(&analytics_url).await;

    let analytics_elements = vec![
        (".chart, .analytics-chart, canvas", "chart"),
        (".date-range, .date-picker", "date range picker"),
        (".metrics-summary, .stats-cards", "metrics summary"),
    ];

    for (selector, name) in analytics_elements {
        let locator = Locator::css(selector);
        match browser.find_element(locator).await {
            Ok(_) => println!("Found: {}", name),
            Err(_) => eprintln!("Not found: {}", name),
        }
    }

    ctx.close().await;
}

#[tokio::test]
pub(crate) async fn test_user_management() {
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

    let users_url = format!("{}/admin/users", ctx.base_url());
    let _ = browser.goto(&users_url).await;

    let user_elements = vec![
        (".user-list, .users-table, #user-list", "user list"),
        (".invite-user, .add-user", "invite button"),
        (".user-row, .user-item, tr.user", "user entries"),
    ];

    for (selector, name) in user_elements {
        let locator = Locator::css(selector);
        match browser.find_element(locator).await {
            Ok(_) => println!("Found: {}", name),
            Err(_) => eprintln!("Not found: {}", name),
        }
    }

    ctx.close().await;
}
