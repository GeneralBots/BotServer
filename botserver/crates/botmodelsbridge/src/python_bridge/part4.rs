use super::*;

impl PythonFaceBridge {
    pub async fn is_running(&self) -> bool {
        let is_healthy = self.is_healthy.read().await;
        *is_healthy
    }

    pub fn get_model(&self) -> &PythonModel {
        &self.config.model
    }

    pub async fn set_model(&mut self, model: PythonModel) -> Result<(), PythonBridgeError> {
        self.stop().await?;
        self.config.model = model;
        self.start().await?;
        Ok(())
    }
}
