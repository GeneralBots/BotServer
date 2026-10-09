use super::*;

impl DatabaseCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<SecretString>,
        host: impl Into<String>,
        port: u16,
        database: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            host: host.into(),
            port,
            database: database.into(),
        }
    }

    pub fn from_url(url: &str) -> Option<Self> {
        let url = url.strip_prefix("postgres://")?;
        let (auth, rest) = url.split_once('@')?;
        let (username, password) = auth.split_once(':')?;
        let (host_port, database) = rest.split_once('/')?;

        let (host, port) = if let Some((h, p)) = host_port.split_once(':') {
            (h.to_string(), p.parse().ok()?)
        } else {
            (host_port.to_string(), 5432)
        };

        let database = database.split('?').next()?.to_string();

        Some(Self {
            username: username.to_string(),
            password: password.parse().unwrap_or_default(),
            host,
            port,
            database,
        })
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn expose_password(&self) -> &str {
        self.password.expose_secret()
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn database(&self) -> &str {
        &self.database
    }

    pub fn to_connection_string(&self) -> SecretString {
        SecretString::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database
        ))
    }

    pub fn to_safe_string(&self) -> String {
        format!(
            "postgres://{}:****@{}:{}/{}",
            self.username, self.host, self.port, self.database
        )
    }
}
impl fmt::Debug for DatabaseCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DatabaseCredentials")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database", &self.database)
            .finish()
    }
}
impl fmt::Display for DatabaseCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_safe_string())
    }
}
