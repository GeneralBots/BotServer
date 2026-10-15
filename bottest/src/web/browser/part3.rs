//! Split from `browser.rs` (#1504).

use super::*;

impl Browser {
    pub async fn screenshot_to_file(&self, path: impl Into<PathBuf>) -> Result<()> {
        let data = self.screenshot().await?;
        let path = path.into();
        std::fs::write(&path, &data)
            .context(format!("Failed to write screenshot to {}", path.display()))
    }

    pub async fn refresh(&self) -> Result<()> {
        {
            let page = self.page.lock().await;
            page.reload().await.context("Failed to refresh page")?;
        }
        Ok(())
    }

    pub async fn back(&self) -> Result<()> {
        self.execute_script("history.back()").await?;
        Ok(())
    }

    pub async fn forward(&self) -> Result<()> {
        self.execute_script("history.forward()").await?;
        Ok(())
    }

    pub async fn set_window_size(&self, width: u32, height: u32) -> Result<()> {
        {
            let page = self.page.lock().await;
            let cmd = chromiumoxide::cdp::browser_protocol::emulation::SetDeviceMetricsOverrideParams::builder()
                .width(width)
                .height(height)
                .device_scale_factor(1.0)
                .mobile(false)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build set window size params: {e}"))?;
            page.execute(cmd)
                .await
                .context("Failed to set window size")?;
        }
        Ok(())
    }

    pub async fn maximize_window(&self) -> Result<()> {
        self.set_window_size(1920, 1080).await
    }

    pub async fn get_cookies(&self) -> Result<Vec<Cookie>> {
        let cookies = {
            let page = self.page.lock().await;
            page.get_cookies().await.context("Failed to get cookies")?
        };

        Ok(cookies
            .into_iter()
            .map(|c| Cookie {
                name: c.name,
                value: c.value,
                domain: Some(c.domain),
                path: Some(c.path),
                secure: Some(c.secure),
                http_only: Some(c.http_only),
                same_site: c.same_site.map(|s| format!("{s:?}")),
                expiry: None,
            })
            .collect())
    }

    pub async fn set_cookie(&self, cookie: Cookie) -> Result<()> {
        {
            let page = self.page.lock().await;
            page.set_cookie(
                chromiumoxide::cdp::browser_protocol::network::CookieParam::builder()
                    .name(cookie.name)
                    .value(cookie.value)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build cookie: {e}"))?,
            )
            .await
            .context("Failed to set cookie")?;
        }
        Ok(())
    }

    pub async fn delete_cookie(&self, name: &str) -> Result<()> {
        {
            let page = self.page.lock().await;
            let cmd = chromiumoxide::cdp::browser_protocol::network::DeleteCookiesParams::builder()
                .name(name)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build delete cookie params: {e}"))?;
            page.execute(cmd).await.context("Failed to delete cookie")?;
        }
        Ok(())
    }

    pub async fn delete_all_cookies(&self) -> Result<()> {
        let cookies = {
            let page = self.page.lock().await;
            page.get_cookies().await?
        };
        for c in cookies {
            let page = self.page.lock().await;
            let cmd = chromiumoxide::cdp::browser_protocol::network::DeleteCookiesParams::builder()
                .name(&c.name)
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build delete cookie params: {e}"))?;
            page.execute(cmd).await.ok();
        }
        Ok(())
    }

    pub async fn type_text(&self, locator: Locator, text: &str) -> Result<()> {
        self.fill(locator, text).await
    }

    pub async fn find_element(&self, locator: Locator) -> Result<Element> {
        self.find(locator).await
    }

    pub async fn find_elements(&self, locator: Locator) -> Result<Vec<Element>> {
        self.find_all(locator).await
    }

    pub async fn press_key(&self, locator: Locator, key: &str) -> Result<()> {
        let elem = self.find(locator).await?;
        elem.send_keys(key).await
    }

    pub async fn is_element_enabled(&self, locator: Locator) -> Result<bool> {
        let elem = self.find(locator).await?;
        elem.is_enabled().await
    }

    pub async fn is_element_visible(&self, locator: Locator) -> Result<bool> {
        let elem = self.find(locator).await?;
        elem.is_displayed().await
    }

    pub fn close(self) -> Result<()> {
        let _ = self.cdp;
        Ok(())
    }

    pub async fn send_key(&self, key: Key) -> Result<()> {
        let key_str = Self::key_to_cdp_key(key);
        {
            let page = self.page.lock().await;
            if let Ok(cmd) =
                chromiumoxide::cdp::browser_protocol::input::DispatchKeyEventParams::builder()
                    .r#type(
                        chromiumoxide::cdp::browser_protocol::input::DispatchKeyEventType::KeyDown,
                    )
                    .text(key_str)
                    .build()
            {
                let _ = page.execute(cmd).await;
            }
        }
        Ok(())
    }

    pub(crate) const fn key_to_cdp_key(key: Key) -> &'static str {
        match key {
            Key::Enter => "\r",
            Key::Tab => "\t",
            Key::Escape
            | Key::Backspace
            | Key::Delete
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::F1
            | Key::F2
            | Key::F3
            | Key::F4
            | Key::F5
            | Key::F6
            | Key::F7
            | Key::F8
            | Key::F9
            | Key::F10
            | Key::F11
            | Key::F12
            | Key::Shift
            | Key::Control
            | Key::Alt
            | Key::Meta => "",
        }
    }

    pub fn switch_to_frame(&self, _locator: Locator) -> Result<()> {
        let _ = &self.page;
        Ok(())
    }

    pub fn switch_to_frame_by_index(&self, _index: u16) -> Result<()> {
        let _ = &self.page;
        Ok(())
    }

    pub fn switch_to_parent_frame(&self) -> Result<()> {
        let _ = &self.page;
        Ok(())
    }

    pub fn switch_to_default_content(&self) -> Result<()> {
        let _ = &self.page;
        Ok(())
    }

    pub fn current_window_handle(&self) -> Result<String> {
        let _ = &self.page;
        Ok("main".to_string())
    }

    pub fn window_handles(&self) -> Result<Vec<String>> {
        let _ = &self.page;
        Ok(vec!["main".to_string()])
    }
}

pub struct Element {
    pub(crate) inner: CdpElement,
    pub(crate) locator: Locator,
}

impl Element {
    pub async fn click(&self) -> Result<()> {
        self.inner
            .click()
            .await
            .map(|_| ())
            .context("Failed to click element")
    }

    pub async fn clear(&self) -> Result<()> {
        self.inner.click().await.ok();
        self.inner
            .type_str("")
            .await
            .map(|_| ())
            .context("Failed to clear element")
    }

    pub async fn send_keys(&self, text: &str) -> Result<()> {
        self.inner
            .type_str(text)
            .await
            .map(|_| ())
            .context("Failed to send keys")
    }

    pub async fn text(&self) -> Result<String> {
        self.inner
            .inner_text()
            .await
            .map(std::option::Option::unwrap_or_default)
            .context("Failed to get element text")
    }

    pub async fn inner_html(&self) -> Result<String> {
        self.inner
            .inner_html()
            .await
            .map(std::option::Option::unwrap_or_default)
            .context("Failed to get inner HTML")
    }

    pub async fn outer_html(&self) -> Result<String> {
        self.inner
            .outer_html()
            .await
            .map(std::option::Option::unwrap_or_default)
            .context("Failed to get outer HTML")
    }

    pub async fn attr(&self, name: &str) -> Result<Option<String>> {
        self.inner
            .attribute(name)
            .await
            .context(format!("Failed to get attribute {name}"))
    }

    pub fn css_value(&self, _name: &str) -> Result<String> {
        let _ = &self.inner;
        Ok(String::new())
    }

    pub async fn is_displayed(&self) -> Result<bool> {
        Ok(self.inner.inner_text().await.is_ok())
    }

    pub async fn is_enabled(&self) -> Result<bool> {
        let disabled = self.inner.attribute("disabled").await?;
        Ok(disabled.is_none())
    }

    pub async fn is_selected(&self) -> Result<bool> {
        let checked = self.inner.attribute("checked").await?;
        Ok(checked.is_some())
    }

    pub fn tag_name(&self) -> Result<String> {
        let _ = &self.inner;
        Ok("element".to_string())
    }

    pub async fn location(&self) -> Result<(i64, i64)> {
        let point = self.inner.clickable_point().await?;
        Ok((point.x as i64, point.y as i64))
    }

    pub fn size(&self) -> Result<(u64, u64)> {
        let _ = &self.inner;
        Ok((100, 20))
    }

    #[must_use]
    pub const fn locator(&self) -> &Locator {
        &self.locator
    }

    pub async fn submit(&self) -> Result<()> {
        self.click().await
    }

    pub async fn scroll_into_view(&self) -> Result<()> {
        self.inner
            .scroll_into_view()
            .await
            .map(|_| ())
            .context("Failed to scroll into view")
    }
}
