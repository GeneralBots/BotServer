use super::*;

impl CsrfLayer {
    pub fn new(manager: Arc<CsrfManager>) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> Arc<CsrfManager> {
        self.manager.clone()
    }
}
