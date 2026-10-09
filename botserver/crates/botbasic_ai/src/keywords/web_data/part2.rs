use super::*;

pub(super) fn register_scrape_images_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["SCRAPE_IMAGES", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?.to_string();
                trace!("SCRAPE_IMAGES {}", url);
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
                    let result = rt.block_on(async { scrape_images(&url).await });
                    let _ = tx.send(result);
                });
                match rx.recv_timeout(Duration::from_secs(30)) {
                    Ok(Ok(result)) => Ok(Dynamic::from(result)),
                    Ok(Err(e)) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        format!("SCRAPE_IMAGES failed: {}", e).into(),
                        Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(EvalAltResult::ErrorRuntime(
                        "SCRAPE_IMAGES timed out".into(),
                        Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("valid SCRAPE_IMAGES syntax registration failed: {e}");
    }

    debug!("Registered SCRAPE_IMAGES keyword");
}
pub(super) async fn fetch_page(url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("Mozilla/5.0 (compatible; BotServer/", env!("CARGO_PKG_VERSION"), ")"))
        .timeout(Duration::from_secs(30))
        .build()?;
    let response = client.get(url).send().await?.text().await?;
    Ok(response)
}
pub(super) async fn scrape_first(
    url: &str,
    selector: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let html = fetch_page(url).await?;
    let document = Html::parse_document(&html);
    let sel = Selector::parse(selector).map_err(|e| format!("Invalid selector: {:?}", e))?;
    if let Some(element) = document.select(&sel).next() {
        let text = element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        return Ok(text);
    }
    Ok(String::new())
}
pub(super) async fn scrape_all(
    url: &str,
    selector: &str,
) -> Result<Array, Box<dyn std::error::Error + Send + Sync>> {
    let html = fetch_page(url).await?;
    let document = Html::parse_document(&html);
    let sel = Selector::parse(selector).map_err(|e| format!("Invalid selector: {:?}", e))?;
    let results: Array = document
        .select(&sel)
        .map(|el| {
            let text = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            Dynamic::from(text)
        })
        .collect();
    Ok(results)
}
pub(super) async fn scrape_table(
    url: &str,
    selector: &str,
) -> Result<Array, Box<dyn std::error::Error + Send + Sync>> {
    let html = fetch_page(url).await?;
    let document = Html::parse_document(&html);
    let table_sel = Selector::parse(selector).map_err(|e| format!("Invalid selector: {:?}", e))?;
    let tr_sel = static_selector("tr", "tr")?;
    let th_sel = static_selector("th", "th")?;
    let td_sel = static_selector("td", "td")?;
    let mut results = Array::new();
    let mut headers: Vec<String> = Vec::new();
    if let Some(table) = document.select(&table_sel).next() {
        for (i, row) in table.select(&tr_sel).enumerate() {
            if i == 0 {
                headers = row
                    .select(&th_sel)
                    .chain(row.select(&td_sel))
                    .map(|cell| cell.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect();
                if headers.is_empty() {}
            } else {
                let mut row_map = Map::new();
                for (j, cell) in row.select(&td_sel).enumerate() {
                    let key = headers
                        .get(j)
                        .cloned()
                        .unwrap_or_else(|| format!("col{}", j));
                    let value = cell.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    row_map.insert(key.into(), Dynamic::from(value));
                }
                if !row_map.is_empty() {
                    results.push(Dynamic::from(row_map));
                }
            }
        }
    }
    Ok(results)
}
pub(super) async fn scrape_links(url: &str) -> Result<Array, Box<dyn std::error::Error + Send + Sync>> {
    let html = fetch_page(url).await?;
    let document = Html::parse_document(&html);
    let sel = static_selector("a[href]", "href")?;
    let base_url = Url::parse(url)?;
    let mut results = Array::new();
    for el in document.select(&sel) {
        if let Some(href) = el.value().attr("href") {
            let absolute = base_url
                .join(href)
                .map(|u| u.to_string())
                .unwrap_or_default();
            if !absolute.is_empty() {
                let mut link = Map::new();
                link.insert("href".into(), Dynamic::from(absolute));
                link.insert(
                    "text".into(),
                    Dynamic::from(el.text().collect::<Vec<_>>().join(" ").trim().to_string()),
                );
                results.push(Dynamic::from(link));
            }
        }
    }
    Ok(results)
}
pub(super) async fn scrape_images(url: &str) -> Result<Array, Box<dyn std::error::Error + Send + Sync>> {
    let html = fetch_page(url).await?;
    let document = Html::parse_document(&html);
    let sel = static_selector("img[src]", "img")?;
    let base_url = Url::parse(url)?;
    let mut results = Array::new();
    for el in document.select(&sel) {
        if let Some(src) = el.value().attr("src") {
            let absolute = base_url
                .join(src)
                .map(|u| u.to_string())
                .unwrap_or_default();
            if !absolute.is_empty() {
                let mut img = Map::new();
                img.insert("src".into(), Dynamic::from(absolute));
                img.insert(
                    "alt".into(),
                    Dynamic::from(el.value().attr("alt").unwrap_or("").to_string()),
                );
                results.push(Dynamic::from(img));
            }
        }
    }
    Ok(results)
}
