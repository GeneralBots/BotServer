use super::*;

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
                base_url: String::new(),
            },
            database: DatabaseConfig {
                url: "postgresql://postgres:postgres@localhost/botserver".to_string(),
                max_connections: 10,
            },
            drive: DriveConfig::default(),
            email: EmailConfig::default(),
            site_path: "/opt/gbo/data".to_string(),
            data_dir: "/opt/gbo/data".to_string(),
        }
    }
}
