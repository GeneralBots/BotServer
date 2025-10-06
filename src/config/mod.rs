use std::env;

#[derive(Clone)]
pub struct AppConfig {
    pub minio: MinioConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub database_custom: DatabaseConfig,
    pub email: EmailConfig,
    pub ai: AIConfig,
    pub site_path: String,
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
pub struct MinioConfig {
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

        let minio = MinioConfig {
            server: env::var("DRIVE_SERVER").expect("DRIVE_SERVER not set"),
            access_key: env::var("DRIVE_ACCESSKEY").expect("DRIVE_ACCESSKEY not set"),
            secret_key: env::var("DRIVE_SECRET").expect("DRIVE_SECRET not set"),
            use_ssl: env::var("DRIVE_USE_SSL")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            bucket: env::var("DRIVE_ORG_PREFIX").unwrap_or_else(|_| "".to_string()),
        };

        let email = EmailConfig {
            from: env::var("EMAIL_FROM").expect("EMAIL_FROM not set"),
            server: env::var("EMAIL_SERVER").expect("EMAIL_SERVER not set"),
            port: env::var("EMAIL_PORT")
                .expect("EMAIL_PORT not set")
                .parse()
                .expect("EMAIL_PORT must be a number"),
            username: env::var("EMAIL_USER").expect("EMAIL_USER not set"),
            password: env::var("EMAIL_PASS").expect("EMAIL_PASS not set"),
        };

        let ai = AIConfig {
            instance: env::var("AI_INSTANCE").expect("AI_INSTANCE not set"),
            key: env::var("AI_KEY").expect("AI_KEY not set"),
            version: env::var("AI_VERSION").expect("AI_VERSION not set"),
            endpoint: env::var("AI_ENDPOINT").expect("AI_ENDPOINT not set"),
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
            site_path: env::var("SITES_ROOT").unwrap()
        }
    }
}