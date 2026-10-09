use super::*;

impl std::fmt::Debug for CaManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaManager")
            .field("config", &self.config)
            .field("ca_params", &self.ca_params.is_some())
            .field("ca_key", &self.ca_key.is_some())
            .field("intermediate_params", &self.intermediate_params.is_some())
            .field("intermediate_key", &self.intermediate_key.is_some())
            .finish()
    }
}
