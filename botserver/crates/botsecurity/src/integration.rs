use anyhow::{Context, Result};
use reqwest::{Certificate, Client, ClientBuilder, Identity};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ServiceUrls {
    pub original: String,
    pub secure: String,
    pub port: u16,
    pub tls_port: u16,
}

#[derive(Debug)]
pub struct TlsIntegration {
    services: HashMap<String, ServiceUrls>,

    ca_cert: Option<Certificate>,

    client_certs: HashMap<String, Identity>,

    tls_enabled: bool,

    https_only: bool,
}

impl TlsIntegration {
    pub fn new(tls_enabled: bool) -> Self {
        let sm = botsecurity_core::get_vault_provider();

        let (qdrant_url, _) = sm
            .as_ref()
            .map(|sm| sm.get_vectordb_config_sync())
            .unwrap_or((String::new(), None));
        let qdrant_secure = qdrant_url.replace("http://", "https://");
        let qdrant_port: u16 = qdrant_url
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(6333);
        let qdrant_tls_port: u16 = qdrant_secure
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(6334);

        let (llm_url, _, _, _, _) = sm.as_ref().map(|sm| sm.get_llm_config()).unwrap_or((
            String::new(),
            String::new(),
            None,
            None,
            String::new(),
        ));
        let llm_secure = llm_url.replace("http://", "https://");
        let llm_port: u16 = llm_url
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8081);
        let llm_tls_port: u16 = llm_secure
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8444);

        let (cache_host, cache_port, _) = sm
            .as_ref()
            .map(|sm| sm.get_cache_config().unwrap_or((String::new(), 6379, None)))
            .unwrap_or((String::new(), 6379, None));
        let cache_tls_port = cache_port + 1;

        let (db_host, db_port, _, _, _) = sm
            .as_ref()
            .map(|sm| sm.get_database_config_sync().unwrap_or((
                String::new(),
                5432,
                String::new(),
                String::new(),
                String::new(),
            )))
            .unwrap_or((
                String::new(),
                5432,
                String::new(),
                String::new(),
                String::new(),
            ));
        let db_tls_port = db_port + 1;

        let (drive_host, _, _) = sm
            .as_ref()
            .map(|sm| sm.get_drive_config().unwrap_or((String::new(), String::new(), String::new())))
            .unwrap_or((String::new(), String::new(), String::new()));
        let drive_port: u16 = drive_host
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(9100);

        let (directory_url, _, _, _) = sm
            .as_ref()
            .map(|sm| sm.get_directory_config_sync())
            .unwrap_or((String::new(), String::new(), String::new(), String::new()));
        let directory_secure = directory_url.replace("http://", "https://");
        let directory_port: u16 = directory_url
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(9000);
        let directory_tls_port: u16 = directory_secure
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8446);

        let mut services = HashMap::new();

        services.insert(
            "api".to_string(),
            ServiceUrls {
                original: String::new(),
                secure: String::new(),
                port: 8080,
                tls_port: 8443,
            },
        );

        services.insert(
            "llm".to_string(),
            ServiceUrls {
                original: llm_url,
                secure: llm_secure,
                port: llm_port,
                tls_port: llm_tls_port,
            },
        );

        services.insert(
            "embedding".to_string(),
            ServiceUrls {
                original: String::new(),
                secure: String::new(),
                port: 8082,
                tls_port: 8445,
            },
        );

        services.insert(
            "qdrant".to_string(),
            ServiceUrls {
                original: qdrant_url,
                secure: qdrant_secure,
                port: qdrant_port,
                tls_port: qdrant_tls_port,
            },
        );

        services.insert(
            "redis".to_string(),
            ServiceUrls {
                original: format!("redis://{}:{}", cache_host, cache_port),
                secure: format!("rediss://{}:{}", cache_host, cache_tls_port),
                port: cache_port,
                tls_port: cache_tls_port,
            },
        );

        services.insert(
            "postgres".to_string(),
            ServiceUrls {
                original: format!("postgres://{}:{}", db_host, db_port),
                secure: format!("postgres://{}:{}?sslmode=require", db_host, db_tls_port),
                port: db_port,
                tls_port: db_tls_port,
            },
        );

        services.insert(
            "minio".to_string(),
            ServiceUrls {
                original: format!("https://{}", drive_host),
                secure: format!("https://{}", drive_host),
                port: drive_port,
                tls_port: drive_port,
            },
        );

        services.insert(
            "directory".to_string(),
            ServiceUrls {
                original: directory_url,
                secure: directory_secure,
                port: directory_port,
                tls_port: directory_tls_port,
            },
        );

        Self {
            services,
            ca_cert: None,
            client_certs: HashMap::new(),
            tls_enabled,
            https_only: tls_enabled,
        }
    }

    pub fn load_ca_cert(&mut self, ca_path: &Path) -> Result<()> {
        if ca_path.exists() {
            let ca_cert_pem = fs::read(ca_path).with_context(|| {
                format!("Failed to read CA certificate from {}", ca_path.display())
            })?;

            let ca_cert =
                Certificate::from_pem(&ca_cert_pem).context("Failed to parse CA certificate")?;

            self.ca_cert = Some(ca_cert);
            info!("Loaded CA certificate from {}", ca_path.display());
        } else {
            warn!("CA certificate not found at {}", ca_path.display());
        }

        Ok(())
    }

    pub fn load_client_cert(
        &mut self,
        service: &str,
        cert_path: &Path,
        key_path: &Path,
    ) -> Result<()> {
        if cert_path.exists() && key_path.exists() {
            let cert = fs::read(cert_path).with_context(|| {
                format!("Failed to read client cert from {}", cert_path.display())
            })?;

            let key = fs::read(key_path).with_context(|| {
                format!("Failed to read client key from {}", key_path.display())
            })?;

            let identity = Identity::from_pem(&[&cert[..], &key[..]].concat())
                .context("Failed to create client identity")?;

            self.client_certs.insert(service.to_string(), identity);
            info!("Loaded client certificate for service: {}", service);
        } else {
            warn!("Client certificate not found for service: {}", service);
        }

        Ok(())
    }

    pub fn convert_url(&self, url: &str) -> String {
        if !self.tls_enabled {
            return url.to_string();
        }

        for urls in self.services.values() {
            if url.starts_with(&urls.original) {
                return url.replace(&urls.original, &urls.secure);
            }
        }

        if url.starts_with("http://") {
            url.replace("http://", "https://")
        } else if url.starts_with("redis://") {
            url.replace("redis://", "rediss://")
        } else {
            url.to_string()
        }
    }

    pub fn get_service_url(&self, service: &str) -> Option<String> {
        self.services.get(service).map(|urls| {
            if self.tls_enabled {
                urls.secure.clone()
            } else {
                urls.original.clone()
            }
        })
    }

    pub fn create_client(&self, service: &str) -> Result<Client> {
        let mut builder = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10));

        if self.tls_enabled {
            builder = builder.use_rustls_tls();

            if let Some(ca_cert) = &self.ca_cert {
                builder = builder.add_root_certificate(ca_cert.clone());
            }

            if let Some(identity) = self.client_certs.get(service) {
                builder = builder.identity(identity.clone());
            }

            if self.https_only {
                builder = builder.https_only(true);
            }
        }

        builder.build().context("Failed to build HTTP client")
    }

    pub fn create_generic_client(&self) -> Result<Client> {
        self.create_client("generic")
    }

    pub fn is_tls_enabled(&self) -> bool {
        self.tls_enabled
    }

    pub fn get_secure_port(&self, service: &str) -> Option<u16> {
        self.services.get(service).map(|urls| {
            if self.tls_enabled {
                urls.tls_port
            } else {
                urls.port
            }
        })
    }

    pub fn update_postgres_url(&self, url: &str) -> String {
        if !self.tls_enabled {
            return url.to_string();
        }

        if url.contains("localhost:5432") || url.contains("127.0.0.1:5432") {
            let base = url
                .replace("localhost:5432", "localhost:5433")
                .replace("127.0.0.1:5432", "127.0.0.1:5433");

            if base.contains("sslmode=") {
                base
            } else if base.contains('?') {
                format!("{}&sslmode=require", base)
            } else {
                format!("{}?sslmode=require", base)
            }
        } else {
            url.to_string()
        }
    }

    pub fn update_redis_url(&self, url: &str) -> String {
        if !self.tls_enabled {
            return url.to_string();
        }

        if url.starts_with("redis://") {
            url.replace("redis://", "rediss://")
                .replace(":6379", ":6380")
        } else {
            url.to_string()
        }
    }

    pub fn load_all_certs_from_dir(&mut self, cert_dir: &Path) -> Result<()> {
        let ca_path = cert_dir.join("ca.crt");
        if ca_path.exists() {
            self.load_ca_cert(&ca_path)?;
        }

        for service in &[
            "api",
            "llm",
            "embedding",
            "qdrant",
            "postgres",
            "redis",
            "minio",
        ] {
            let service_dir = cert_dir.join(service);
            if service_dir.exists() {
                let cert_path = service_dir.join("client.crt");
                let key_path = service_dir.join("client.key");

                if cert_path.exists() && key_path.exists() {
                    self.load_client_cert(service, &cert_path, &key_path)?;
                }
            }
        }

        Ok(())
    }
}

static TLS_INTEGRATION: OnceLock<Arc<TlsIntegration>> = OnceLock::new();

pub fn init_tls_integration(tls_enabled: bool, cert_dir: Option<PathBuf>) -> Result<()> {
    let _ = TLS_INTEGRATION.get_or_init(|| {
        let mut integration = TlsIntegration::new(tls_enabled);

        if tls_enabled {
            if let Some(dir) = cert_dir {
                if let Err(e) = integration.load_all_certs_from_dir(&dir) {
                    warn!("Failed to load some certificates: {}", e);
                }
            }
        }

        info!("TLS integration initialized (TLS: {})", tls_enabled);
        Arc::new(integration)
    });

    Ok(())
}

pub fn get_tls_integration() -> Option<Arc<TlsIntegration>> {
    TLS_INTEGRATION.get().cloned()
}

pub fn to_secure_url(url: &str) -> String {
    if let Some(integration) = get_tls_integration() {
        integration.convert_url(url)
    } else {
        url.to_string()
    }
}

pub fn create_https_client(service: &str) -> Result<Client> {
    if let Some(integration) = get_tls_integration() {
        integration.create_client(service)
    } else {
        Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to build default HTTP client")
    }
}
