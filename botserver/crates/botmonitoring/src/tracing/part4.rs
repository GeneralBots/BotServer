use super::*;

impl Default for ResourceAttributes {
    fn default() -> Self {
        Self {
            service_name: "botserver".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            service_instance_id: Uuid::new_v4().to_string(),
            host_name: std::env::var("HOSTNAME").ok(),
            host_type: None,
            os_type: Some(std::env::consts::OS.to_string()),
            deployment_environment: std::env::var("DEPLOYMENT_ENV").ok(),
            custom: HashMap::new(),
        }
    }
}
