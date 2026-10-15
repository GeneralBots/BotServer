//! Split from `browser.rs` (#1504).

use super::*;

impl Browser {
    pub async fn new(config: BrowserConfig) -> Result<Self> {
        log::info!("Connecting to browser CDP on port {}", config.debug_port);

        let json_url = format!("http://127.0.0.1:{}/json/version", config.debug_port);
        let ws_url = match reqwest::get(&json_url).await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp
                    .json()
                    .await
                    .context("Failed to parse CDP JSON response")?;
                json.get("webSocketDebuggerUrl")
                    .and_then(|v| v.as_str())
                    .map_or_else(
                        || format!("ws://127.0.0.1:{}", config.debug_port),
                        std::string::ToString::to_string,
                    )
            }
            _ => format!("ws://127.0.0.1:{}", config.debug_port),
        };

        log::info!("CDP WebSocket URL: {ws_url}");

        let (browser, mut handler) = CdpBrowser::connect(&ws_url)
            .await
            .context(format!("Failed to connect to browser CDP at {ws_url}"))?;

        let handle = tokio::spawn(async move {
            loop {
                match handler.next().await {
                    Some(Ok(())) => {}
                    Some(Err(e)) => {
                        let err_str = format!("{e:?}");
                        if err_str.contains("did not match any variant") {
                            log::trace!("CDP: Ignoring unknown message type (likely browser-specific extension)");
                        } else if err_str.contains("ResetWithoutClosingHandshake")
                            || err_str.contains("AlreadyClosed")
                        {
                            log::debug!("CDP connection closed: {e:?}");
                            break;
                        } else {
                            log::debug!("CDP handler error: {e:?}");
                        }
                    }
                    None => {
                        log::debug!("CDP handler stream ended");
                        break;
                    }
                }
            }
        });

        let page = match browser.pages().await {
            Ok(pages) if !pages.is_empty() => {
                log::info!("Using existing page");
                pages.into_iter().next().unwrap()
            }
            _ => {
                log::info!("Creating new page");
                browser
                    .new_page("about:blank")
                    .await
                    .context("Failed to create new page")?
            }
        };

        let _ = page.bring_to_front().await;

        let _ = page.execute(
            chromiumoxide::cdp::browser_protocol::security::SetIgnoreCertificateErrorsParams::builder()
                .ignore(true)
                .build()
                .unwrap()
        ).await;
        log::info!("CDP: Set to ignore certificate errors");

        if let Ok(cmd) = chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
            .width(config.window_width)
            .height(config.window_height)
            .device_scale_factor(1.0)
            .mobile(false)
            .build()
        {
            let _ = page.execute(cmd).await;
        }

        log::info!("Successfully connected to browser via CDP");

        Ok(Self {
            cdp: Arc::new(browser),
            page: Arc::new(Mutex::new(page)),
            config,
            _handle: handle,
        })
    }

    pub async fn launch(config: BrowserConfig) -> Result<Self> {
        log::info!("Launching new browser with CDP");

        let cdp_config = config.build_cdp_config()?;

        let (browser, mut handler) = CdpBrowser::launch(cdp_config)
            .await
            .context("Failed to launch browser")?;

        let handle = tokio::spawn(async move {
            loop {
                match handler.next().await {
                    Some(Ok(())) => {}
                    Some(Err(e)) => {
                        let err_str = format!("{e:?}");
                        if err_str.contains("did not match any variant") {
                            log::trace!("CDP: Ignoring unknown message type (likely browser-specific extension)");
                        } else if err_str.contains("ResetWithoutClosingHandshake")
                            || err_str.contains("AlreadyClosed")
                        {
                            log::debug!("CDP connection closed: {e:?}");
                            break;
                        } else {
                            log::debug!("CDP handler error: {e:?}");
                        }
                    }
                    None => {
                        log::debug!("CDP handler stream ended");
                        break;
                    }
                }
            }
        });

        let page = browser
            .new_page("about:blank")
            .await
            .context("Failed to create new page")?;

        if let Ok(cmd) = chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
            .width(config.window_width)
            .height(config.window_height)
            .device_scale_factor(1.0)
            .mobile(false)
            .build()
        {
            let _ = page.execute(cmd).await;
        }

        log::info!("Browser launched successfully");

        Ok(Self {
            cdp: Arc::new(browser),
            page: Arc::new(Mutex::new(page)),
            config,
            _handle: handle,
        })
    }

    pub async fn new_headless() -> Result<Self> {
        Self::launch(BrowserConfig::default().headless(true)).await
    }

    pub async fn new_headed() -> Result<Self> {
        Self::launch(BrowserConfig::default().headless(false)).await
    }

    pub async fn goto(&self, url: &str) -> Result<()> {
        if url.starts_with("https://") {
            log::info!("Using JavaScript navigation for HTTPS URL: {url}");

            {
                let page = self.page.lock().await;
                let _ = page.bring_to_front().await;
                let _ = page.goto("about:blank").await;
            }
            sleep(Duration::from_millis(100)).await;

            {
                let page = self.page.lock().await;
                let nav_script = format!("window.location.href = '{url}';");
                let _ = page.evaluate(nav_script.as_str()).await;
            }

            sleep(Duration::from_millis(1500)).await;

            {
                let page = self.page.lock().await;
                if let Ok(Some(current)) = page.url().await {
                    if current.as_str() != "about:blank" {
                        log::info!("Navigation successful: {current}");
                    }
                }
            }
        } else {
            {
                let page = self.page.lock().await;
                let _ = page.bring_to_front().await;
                page.goto(url)
                    .await
                    .context(format!("Failed to navigate to {url}"))?;
            }

            sleep(Duration::from_millis(300)).await;
        }

        {
            let page = self.page.lock().await;
            let _ = page.bring_to_front().await;
            let _ = page
                .evaluate("window.focus(); document.body.style.visibility = 'visible';")
                .await;
        }

        Ok(())
    }

    pub async fn current_url(&self) -> Result<String> {
        let url = {
            let page = self.page.lock().await;
            page.url()
                .await
                .context("Failed to get current URL")?
                .unwrap_or_default()
        };
        Ok(url)
    }

    pub async fn title(&self) -> Result<String> {
        let title = {
            let page = self.page.lock().await;
            page.get_title()
                .await
                .context("Failed to get page title")?
                .unwrap_or_default()
        };
        Ok(title)
    }

    pub async fn page_source(&self) -> Result<String> {
        let content = {
            let page = self.page.lock().await;
            page.content().await.context("Failed to get page source")?
        };
        Ok(content)
    }

    pub async fn find(&self, locator: Locator) -> Result<Element> {
        let element = {
            let page = self.page.lock().await;
            let selector = locator.to_css_selector();
            page.find_element(&selector)
                .await
                .context(format!("Failed to find element: {locator:?}"))?
        };

        Ok(Element {
            inner: element,
            locator,
        })
    }

    pub async fn find_all(&self, locator: Locator) -> Result<Vec<Element>> {
        let elements = {
            let page = self.page.lock().await;
            let selector = locator.to_css_selector();
            page.find_elements(&selector)
                .await
                .context(format!("Failed to find elements: {locator:?}"))?
        };

        Ok(elements
            .into_iter()
            .map(|e| Element {
                inner: e,
                locator: locator.clone(),
            })
            .collect())
    }

    pub async fn wait_for(&self, locator: Locator) -> Result<Element> {
        self.wait_for_condition(locator, WaitCondition::Present)
            .await
    }

    pub async fn wait_for_condition(
        &self,
        locator: Locator,
        condition: WaitCondition,
    ) -> Result<Element> {
        let timeout = self.config.timeout;
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match &condition {
                WaitCondition::Present | WaitCondition::Visible | WaitCondition::Clickable => {
                    if let Ok(elem) = self.find(locator.clone()).await {
                        return Ok(elem);
                    }
                }
                WaitCondition::NotPresent => {
                    if self.find(locator.clone()).await.is_err() {
                        anyhow::bail!("Element not present (expected)");
                    }
                }
                WaitCondition::NotVisible => {
                    if self.find(locator.clone()).await.is_err() {
                        anyhow::bail!("Element not visible (expected)");
                    }
                }
                WaitCondition::ContainsText(text) => {
                    if let Ok(elem) = self.find(locator.clone()).await {
                        if let Ok(elem_text) = elem.text().await {
                            if elem_text.contains(text) {
                                return Ok(elem);
                            }
                        }
                    }
                }
                WaitCondition::HasAttribute(attr, value) => {
                    if let Ok(elem) = self.find(locator.clone()).await {
                        if let Ok(Some(attr_val)) = elem.attr(attr).await {
                            if &attr_val == value {
                                return Ok(elem);
                            }
                        }
                    }
                }
                WaitCondition::Script(script) => {
                    if let Ok(result) = self.execute_script(script).await {
                        if result.as_bool().unwrap_or(false) {
                            return self.find(locator).await;
                        }
                    }
                }
            }

            sleep(Duration::from_millis(100)).await;
        }

        anyhow::bail!("Timeout waiting for element {locator:?} with condition {condition:?}")
    }

    pub async fn click(&self, locator: Locator) -> Result<()> {
        let elem = self
            .wait_for_condition(locator, WaitCondition::Clickable)
            .await?;
        elem.click().await
    }

    pub async fn fill(&self, locator: Locator, text: &str) -> Result<()> {
        let elem = self
            .wait_for_condition(locator, WaitCondition::Visible)
            .await?;
        elem.clear().await?;
        elem.send_keys(text).await
    }

    pub async fn text(&self, locator: Locator) -> Result<String> {
        let elem = self.find(locator).await?;
        elem.text().await
    }

    pub async fn exists(&self, locator: Locator) -> bool {
        self.find(locator).await.is_ok()
    }

    pub async fn execute_script(&self, script: &str) -> Result<serde_json::Value> {
        let result = {
            let page = self.page.lock().await;
            page.evaluate(script)
                .await
                .context("Failed to execute script")?
        };
        Ok(result.value().cloned().unwrap_or(serde_json::Value::Null))
    }

    pub async fn execute_script_with_args(
        &self,
        script: &str,
        _args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        self.execute_script(script).await
    }

    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        let screenshot = {
            let page = self.page.lock().await;
            page.screenshot(
                chromiumoxide::page::ScreenshotParams::builder()
                    .format(CaptureScreenshotFormat::Png)
                    .build(),
            )
            .await
            .context("Failed to take screenshot")?
        };
        Ok(screenshot)
    }
}
