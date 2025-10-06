use thirtyfour::{ChromeCapabilities, ChromiumLikeCapabilities, WebDriver};
use tokio::sync::{Semaphore, SemaphorePermit};
use std::sync::Arc;

pub struct BrowserPool {
    semaphore: Arc<Semaphore>,
    webdriver_url: String,
}

impl BrowserPool {
    pub async fn new(max_browsers: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let webdriver_url = std::env::var("WEBDRIVER_URL")
            .unwrap_or_else(|_| "http://localhost:9515".to_string());
            
        Ok(Self {
            semaphore: Arc::new(Semaphore::new(max_browsers)),
            webdriver_url,
        })
    }

    pub async fn get_browser(&self) -> Result<(WebDriver, SemaphorePermit<'_>), Box<dyn std::error::Error>> {
        let permit = self.semaphore.acquire().await?;
        
        let mut caps = ChromeCapabilities::new();
        caps.add_arg("--headless=new")?;
        caps.add_arg("--no-sandbox")?;
        caps.add_arg("--disable-dev-shm-usage")?;

        let driver = WebDriver::new(&self.webdriver_url, caps).await?;
        Ok((driver, permit))
    }

    pub async fn with_browser<F, T>(&self, f: F) -> Result<T, Box<dyn std::error::Error>>
    where
        F: FnOnce(WebDriver) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, Box<dyn std::error::Error>>> + Send>>,
    {
        let (driver, _permit) = self.get_browser().await?;
        let result = f(driver).await;
        
        if let Ok(driver) = result.as_ref().map(|_| &driver) {
            let _ = driver.quit().await;
        }
        
        result
    }
}
