use super::*;

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            browser_type: BrowserType::Chrome,
            debug_port: 9222,
            headless: true,
            window_width: 1920,
            window_height: 1080,
            timeout: Duration::from_secs(30),
        }
    }
}
impl BrowserConfig {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub const fn with_browser(mut self, browser: BrowserType) -> Self {
        self.browser_type = browser;
        self
    }

    pub const fn with_debug_port(mut self, port: u16) -> Self {
        self.debug_port = port;
        self
    }

    pub const fn headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    pub const fn with_window_size(mut self, width: u32, height: u32) -> Self {
        self.window_width = width;
        self.window_height = height;
        self
    }

    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}
