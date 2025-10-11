use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub minio: DriveConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub database_custom: DatabaseConfig,
    pub email: EmailConfig,
    pub ai: AIConfig,
    pub site_path: String,
    pub s3_bucket: String,
}

#[derive(Clone)]
pub struct DatabaseConfig {
    pub username: String,
    pub password: String,
    pub server: String,
    pub port: u32,
    pub database: String,
}

#[derive(Clone)]
pub struct DriveConfig {
    pub server: String,
    pub access_key: String,
    pub secret_key: String,
    pub use_ssl: bool,
    pub bucket: String,
}

#[derive(Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Clone)]
pub struct EmailConfig {
    pub from: String,
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub struct AIConfig {
    pub instance: String,
    pub key: String,
    pub version: String,
    pub endpoint: String,
}

impl AppConfig {
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.username,
            self.database.password,
            self.database.server,
            self.database.port,
            self.database.database
        )
    }

    pub fn database_custom_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database_custom.username,
            self.database_custom.password,
            self.database_custom.server,
            self.database_custom.port,
            self.database_custom.database
        )
    }

    pub fn from_env() -> Self {
        let database = DatabaseConfig {
            username: env::var("TABLES_USERNAME").unwrap_or_else(|_| "user".to_string()),
            password: env::var("TABLES_PASSWORD").unwrap_or_else(|_| "pass".to_string()),
            server: env::var("TABLES_SERVER").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("TABLES_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5432),
            database: env::var("TABLES_DATABASE").unwrap_or_else(|_| "db".to_string()),
        };

        let database_custom = DatabaseConfig {
            username: env::var("CUSTOM_USERNAME").unwrap_or_else(|_| "user".to_string()),
            password: env::var("CUSTOM_PASSWORD").unwrap_or_else(|_| "pass".to_string()),
            server: env::var("CUSTOM_SERVER").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("CUSTOM_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(5432),
            database: env::var("CUSTOM_DATABASE").unwrap_or_else(|_| "db".to_string()),
        };

        let minio = DriveConfig {
            server: env::var("DRIVE_SERVER").unwrap_or_else(|_| "localhost:9000".to_string()),
            access_key: env::var("DRIVE_ACCESSKEY").unwrap_or_else(|_| "minioadmin".to_string()),
            secret_key: env::var("DRIVE_SECRET").unwrap_or_else(|_| "minioadmin".to_string()),
            use_ssl: env::var("DRIVE_USE_SSL")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            bucket: env::var("DRIVE_ORG_PREFIX").unwrap_or_else(|_| "botserver".to_string()),
        };

        let email = EmailConfig {
            from: env::var("EMAIL_FROM").unwrap_or_else(|_| "noreply@example.com".to_string()),
            server: env::var("EMAIL_SERVER").unwrap_or_else(|_| "smtp.example.com".to_string()),
            port: env::var("EMAIL_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            username: env::var("EMAIL_USER").unwrap_or_else(|_| "user".to_string()),
            password: env::var("EMAIL_PASS").unwrap_or_else(|_| "pass".to_string()),
        };

        let ai = AIConfig {
            instance: env::var("AI_INSTANCE").unwrap_or_else(|_| "gpt-4".to_string()),
            key: env::var("AI_KEY").unwrap_or_else(|_| "key".to_string()),
            version: env::var("AI_VERSION").unwrap_or_else(|_| "2023-12-01-preview".to_string()),
            endpoint: env::var("AI_ENDPOINT")
                .unwrap_or_else(|_| "https://api.openai.com".to_string()),
        };

        AppConfig {
            minio,
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
                port: env::var("SERVER_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(8080),
            },
            database,
            database_custom,
            email,
            ai,
            s3_bucket: env::var("DRIVE_BUCKET").unwrap_or_else(|_| "default".to_string()),

            site_path: env::var("SITES_ROOT").unwrap_or_else(|_| "./sites".to_string()),
        }
    }
}
