use super::*;

/// Parse a hard-coded, known-valid CSS selector, propagating failure via
/// `Result` (#1368 — never panic in the server). The patterns passed here
/// are constants; an error means the constant itself is malformed.
pub(super) fn static_selector(pattern: &str, label: &str) -> Result<Selector, String> {
    Selector::parse(pattern).map_err(|e| format!("static {label} selector invalid: {e:?}"))
}
pub fn register_web_data_keywords(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_rss_keyword(state.clone(), user.clone(), engine);
    register_scrape_keyword(state.clone(), user.clone(), engine);
    register_scrape_all_keyword(state.clone(), user.clone(), engine);
    register_scrape_table_keyword(state.clone(), user.clone(), engine);
    register_scrape_links_keyword(state.clone(), user.clone(), engine);
    register_scrape_images_keyword(state, user, engine);
}
pub(super) fn register_rss_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(["RSS", "$expr$"], false, move |context, inputs| {
            let url = context.eval_expression_tree(&inputs[0])?.to_string();
            trace!("RSS {}", url);
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        let err: Box<dyn std::error::Error + Send + Sync> = format!("Failed to create runtime: {}", e).into();
                        let _ = tx.send(Err(err));
                        return;
                    }
                };
                let result = rt.block_on(async { fetch_rss(&url, 100).await });
                let _ = tx.send(result);
            });
            match rx.recv_timeout(Duration::from_secs(30)) {
                Ok(Ok(result)) => Ok(Dynamic::from(result)),
                Ok(Err(e)) => Err(Box::new(EvalAltResult::ErrorRuntime(
                    format!("RSS failed: {}", e).into(),
                    Position::NONE,
                ))),
                Err(_) => Err(Box::new(EvalAltResult::ErrorRuntime(
                    "RSS timed out".into(),
                    Position::NONE,
                ))),
            }
        })
{
    log::error!("Failed to register the custom syntax: {e}");
}

    if let Err(e) = engine
        .register_custom_syntax(
            ["RSS", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                let limit = context
                    .eval_expression_tree(&inputs[1])?
                    .as_int()
                    .unwrap_or(10) as usize;
                trace!("RSS {} limit {}", url, limit);
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let err: Box<dyn std::error::Error + Send + Sync> = format!("Failed to create runtime: {}", e).into();
                            let _ = tx.send(Err(err));
                            return;
                        }
                    };
                    let result = rt.block_on(async { fetch_rss(&url, limit).await });
                    let _ = tx.send(result);
                });
                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(Ok(result)) => Ok(Dynamic::from(result)),
                    Ok(Err(e)) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        format!("RSS failed: {}", e).into(),
                        Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        "RSS timed out".into(),
                        Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("valid RSS syntax registration failed: {e}");
    }

    debug!("Registered RSS keyword");
}
pub(super) async fn fetch_rss(
    url: &str,
    limit: usize,
) -> Result<Array, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("BotServer/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()?;
    let content = client.get(url).send().await?.bytes().await?;
    let channel = rss::Channel::read_from(&content[..])?;
    let mut results = Array::new();
    for item in channel.items().iter().take(limit) {
        let mut entry = Map::new();
        entry.insert(
            "title".into(),
            Dynamic::from(item.title().unwrap_or("").to_string()),
        );
        entry.insert(
            "link".into(),
            Dynamic::from(item.link().unwrap_or("").to_string()),
        );
        entry.insert(
            "description".into(),
            Dynamic::from(item.description().unwrap_or("").to_string()),
        );
        entry.insert(
            "pubDate".into(),
            Dynamic::from(item.pub_date().unwrap_or("").to_string()),
        );
        entry.insert(
            "author".into(),
            Dynamic::from(item.author().unwrap_or("").to_string()),
        );
        if let Some(guid) = item.guid() {
            entry.insert("guid".into(), Dynamic::from(guid.value().to_string()));
        }
        results.push(Dynamic::from(entry));
    }
    Ok(results)
}
pub(super) fn register_scrape_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["SCRAPE", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                let selector = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("SCRAPE {} selector {}", url, selector);
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let err: Box<dyn std::error::Error + Send + Sync> = format!("Failed to create runtime: {}", e).into();
                            let _ = tx.send(Err(err));
                            return;
                        }
                    };
                    let result = rt.block_on(async { scrape_first(&url, &selector).await });
                    let _ = tx.send(result);
                });
                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(Ok(result)) => Ok(Dynamic::from(result)),
                    Ok(Err(e)) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        format!("SCRAPE failed: {}", e).into(),
                        Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        "SCRAPE timed out".into(),
                        Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("valid SCRAPE syntax registration failed: {e}");
    }

    debug!("Registered SCRAPE keyword");
}
pub(super) fn register_scrape_all_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["SCRAPE_ALL", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                let selector = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("SCRAPE_ALL {} selector {}", url, selector);
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let err: Box<dyn std::error::Error + Send + Sync> = format!("Failed to create runtime: {}", e).into();
                            let _ = tx.send(Err(err));
                            return;
                        }
                    };
                    let result = rt.block_on(async { scrape_all(&url, &selector).await });
                    let _ = tx.send(result);
                });
                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(Ok(result)) => Ok(Dynamic::from(result)),
                    Ok(Err(e)) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        format!("SCRAPE_ALL failed: {}", e).into(),
                        Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        "SCRAPE_ALL timed out".into(),
                        Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("valid SCRAPE_ALL syntax registration failed: {e}");
    }

    debug!("Registered SCRAPE_ALL keyword");
}
pub(super) fn register_scrape_table_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["SCRAPE_TABLE", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                let selector = context.eval_expression_tree(&inputs[1])?.to_string();
                trace!("SCRAPE_TABLE {} selector {}", url, selector);
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let err: Box<dyn std::error::Error + Send + Sync> = format!("Failed to create runtime: {}", e).into();
                            let _ = tx.send(Err(err));
                            return;
                        }
                    };
                    let result = rt.block_on(async { scrape_table(&url, &selector).await });
                    let _ = tx.send(result);
                });
                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(Ok(result)) => Ok(Dynamic::from(result)),
                    Ok(Err(e)) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        format!("SCRAPE_TABLE failed: {}", e).into(),
                        Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        "SCRAPE_TABLE timed out".into(),
                        Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("valid SCRAPE_TABLE syntax registration failed: {e}");
    }

    debug!("Registered SCRAPE_TABLE keyword");
}
pub(super) fn register_scrape_links_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["SCRAPE_LINKS", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                trace!("SCRAPE_LINKS {}", url);
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let rt = match tokio::runtime::Runtime::new() {
                        Ok(rt) => rt,
                        Err(e) => {
                            let err: Box<dyn std::error::Error + Send + Sync> = format!("Failed to create runtime: {}", e).into();
                            let _ = tx.send(Err(err));
                            return;
                        }
                    };
                    let result = rt.block_on(async { scrape_links(&url).await });
                    let _ = tx.send(result);
                });
                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(Ok(result)) => Ok(Dynamic::from(result)),
                    Ok(Err(e)) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        format!("SCRAPE_LINKS failed: {}", e).into(),
                        Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        "SCRAPE_LINKS timed out".into(),
                        Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("valid SCRAPE_LINKS syntax registration failed: {e}");
    }

    debug!("Registered SCRAPE_LINKS keyword");
}
