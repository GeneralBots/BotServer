use thirtyfour::{DesiredCapabilities, WebDriver};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BrowserPool {
    drivers: Arc<Mutex<Vec<WebDriver>>>,
    brave_path: String,
}

impl BrowserPool {
    pub fn new() -> Self {
        Self {
            drivers: Arc::new(Mutex::new(Vec::new())),
            brave_path: "/usr/bin/brave-browser".to_string(),
        }
    }

    pub async fn get_driver(&self) -> Result<WebDriver, Box<dyn std::error::Error + Send + Sync>> {
        let mut caps = DesiredCapabilities::chrome();
        
        // Use add_arg instead of add_chrome_arg
        caps.add_arg("--disable-gpu")?;
        caps.add_arg("--no-sandbox")?;
        caps.add_arg("--disable-dev-shm-usage")?;

        let driver = WebDriver::new("http://localhost:9515", caps).await?;
        
        let mut drivers = self.drivers.lock().await;
        drivers.push(driver.clone());
        
        Ok(driver)
    }

    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut drivers = self.drivers.lock().await;
        for driver in drivers.iter() {
            let _ = driver.quit().await;
        }
        drivers.clear();
        Ok(())
    }
}

impl Default for BrowserPool {
    fn default() -> Self {
        Self::new()
    }
}
