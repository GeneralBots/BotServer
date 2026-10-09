use super::*;

impl AppConfig {
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "server.host" => Some(self.server.host.clone()),
            "server.port" => Some(self.server.port.to_string()),
            "server.base_url" => Some(self.server.base_url.clone()),
            "database.url" => Some(self.database.url.clone()),
            "drive.bucket" => Some(self.drive.bucket.clone()),
            "site_path" => Some(self.site_path.clone()),
            "data_dir" => Some(self.data_dir.clone()),
            _ => None,
        }
    }

    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::default())
    }

    pub fn from_database(
        _pool: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::from_env()
    }

pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
    let drive = match DriveConfig::from_vault() {
        Ok(d) => d,
        Err(vault_err) => {
            log::warn!("DriveConfig::from_vault() failed: {vault_err}, trying env");
            match DriveConfig::from_env() {
                Ok(d) => d,
                Err(env_err) => {
                    log::warn!("DriveConfig::from_env() also failed: {env_err} — starting without Drive");
                    DriveConfig::default()
                }
            }
        }
    };

    Ok(Self {
        server: ServerConfig {
            host: "0.0.0.0".to_string(),
            port: std::env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            base_url: String::new(),
        },
        database: DatabaseConfig {
            url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/botserver".to_string()),
            max_connections: 10,
        },
        drive,
        email: EmailConfig::default(),
        site_path: std::env::var("SITE_PATH")
            .unwrap_or_else(|_| "/opt/gbo/data".to_string()),
        data_dir: std::env::var("DATA_DIR")
            .unwrap_or_else(|_| "/opt/gbo/data".to_string()),
    })
}
}
