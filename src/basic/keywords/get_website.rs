use crate::{shared::state::AppState, shared::models::UserSession, web_automation::BrowserPool};
use headless_chrome::browser::tab::Tab;
use log::info;
use rhai::{Dynamic, Engine};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub fn get_website_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let browser_pool = state.browser_pool.clone();

    engine
        .register_custom_syntax(
            &["WEBSITE", "OF", "$expr$"],
            false,
            move |context, inputs| {
                let search_term = context.eval_expression_tree(&inputs[0])?.to_string();

                info!("GET WEBSITE executed - Search: '{}'", search_term);

                let browser_pool_clone = browser_pool.clone();
                let fut = execute_headless_browser_search(browser_pool_clone, &search_term);

                let result =
                    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
                        .map_err(|e| format!("Headless browser search failed: {}", e))?;

                Ok(Dynamic::from(result))
            },
        )
        .unwrap();
}

pub async fn execute_headless_browser_search(
    browser_pool: Arc<BrowserPool>,
    search_term: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting headless browser search: '{}' ", search_term);

    let term = search_term.to_string();

    let result = browser_pool
        .with_browser(move |tab| {
            let term = term.clone();
            Box::pin(async move { perform_search(tab, &term).await })
        })
        .await?;

    Ok(result)
}

async fn perform_search(
    tab: Arc<Tab>,
    search_term: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    tab.navigate_to("https://duckduckgo.com")
        .map_err(|e| format!("Failed to navigate: {}", e))?;

    tab.wait_for_element("#searchbox_input")
        .map_err(|e| format!("Failed to find search box: {}", e))?;

    let search_input = tab
        .find_element("#searchbox_input")
        .map_err(|e| format!("Failed to find search input: {}", e))?;

    search_input
        .click()
        .map_err(|e| format!("Failed to click search input: {}", e))?;

    search_input
        .type_into(search_term)
        .map_err(|e| format!("Failed to type into search input: {}", e))?;

    search_input
        .press_key("Enter")
        .map_err(|e| format!("Failed to press Enter: {}", e))?;

    sleep(Duration::from_millis(3000)).await;

    let _ = tab.wait_for_element("[data-testid='result']");

    let results = extract_search_results(&tab).await?;

    if !results.is_empty() {
        Ok(results[0].clone())
    } else {
        Ok("No results found".to_string())
    }
}

async fn extract_search_results(
    tab: &Arc<Tab>,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let mut results = Vec::new();

    let selectors = [
        "a[data-testid='result-title-a']",
        "a[data-testid='result-extras-url-link']",
        "a.eVNpHGjtxRBq_gLOfGDr",
        "a.Rn_JXVtoPVAFyGkcaXyK",
        ".ikg2IXiCD14iVX7AdZo1 a",
        ".OQ_6vPwNhCeusNiEDcGp a",
        ".result__a",
        "a.result-link",
        ".result a[href]",
    ];

    for selector in &selectors {
        if let Ok(elements) = tab.find_elements(selector) {
            for element in elements {
                if let Ok(Some(href)) = element.get_attribute_value("href") {
                    if href.starts_with("http")
                        && !href.contains("duckduckgo.com")
                        && !href.contains("duck.co")
                        && !results.contains(&href)
                    {
                        let display_text = element.get_inner_text().unwrap_or_default();

                        if !display_text.is_empty() && !display_text.contains("Ad") {
                            results.push(href);
                        }
                    }
                }
            }
            if !results.is_empty() {
                break;
            }
        }
    }

    results.dedup();

    Ok(results)
}
