use super::*;

impl CaManager {
    pub fn new(config: CaConfig) -> Result<Self> {
        let mut manager = Self {
            config,
            ca_params: None,
            ca_key: None,
            intermediate_params: None,
            intermediate_key: None,
        };

        manager.load_ca()?;

        Ok(manager)
    }

    pub fn init_ca(&mut self) -> Result<()> {
        info!("Initializing new Certificate Authority");

        self.create_ca_directories()?;

        self.generate_root_ca()?;

        if self.config.intermediate_cert_path.is_some() {
            self.generate_intermediate_ca()?;
        }

        info!("Certificate Authority initialized successfully");
        Ok(())
    }

    pub(super) fn load_ca(&mut self) -> Result<()> {
        if self.config.ca_cert_path.exists() && self.config.ca_key_path.exists() {
            debug!("Loading existing CA from {:?}", self.config.ca_cert_path);

            let key_pem = fs::read_to_string(&self.config.ca_key_path)?;
            let key_pair = KeyPair::from_pem(&key_pem)?;

            let mut params = CertificateParams::default();
            params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

            let mut dn = DistinguishedName::new();
            dn.push(DnType::CountryName, &self.config.country);
            dn.push(DnType::OrganizationName, &self.config.organization);
            dn.push(DnType::CommonName, "BotServer Root CA");
            params.distinguished_name = dn;

            self.ca_params = Some(params);
            self.ca_key = Some(key_pair);

            if let (Some(cert_path), Some(key_path)) = (
                &self.config.intermediate_cert_path,
                &self.config.intermediate_key_path,
            ) {
                if cert_path.exists() && key_path.exists() {
                    let key_pem = fs::read_to_string(key_path)?;
                    let key_pair = KeyPair::from_pem(&key_pem)?;

                    let mut params = CertificateParams::default();
                    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));

                    let mut dn = DistinguishedName::new();
                    dn.push(DnType::CountryName, &self.config.country);
                    dn.push(DnType::OrganizationName, &self.config.organization);
                    dn.push(DnType::CommonName, "BotServer Intermediate CA");
                    params.distinguished_name = dn;

                    self.intermediate_params = Some(params);
                    self.intermediate_key = Some(key_pair);
                }
            }

            info!("Loaded existing CA certificates");
        } else {
            warn!("No existing CA found, initialization required");
        }

        Ok(())
    }

    pub(super) fn generate_root_ca(&mut self) -> Result<()> {
        let mut params = CertificateParams::default();

        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CountryName, &self.config.country);
        dn.push(DnType::StateOrProvinceName, &self.config.state);
        dn.push(DnType::LocalityName, &self.config.locality);
        dn.push(DnType::OrganizationName, &self.config.organization);
        dn.push(DnType::CommonName, "BotServer Root CA");
        params.distinguished_name = dn;

        params.not_before = OffsetDateTime::now_utc();
        params.not_after =
            OffsetDateTime::now_utc() + Duration::days(self.config.validity_days * 2);

        let key_pair = KeyPair::generate()?;

        let cert = params.self_signed(&key_pair)?;

        fs::write(&self.config.ca_cert_path, cert.pem())?;
        fs::write(&self.config.ca_key_path, key_pair.serialize_pem())?;

        self.ca_params = Some(params);
        self.ca_key = Some(key_pair);

        info!("Generated root CA certificate");
        Ok(())
    }

    pub(super) fn generate_intermediate_ca(&mut self) -> Result<()> {
        let ca_params = self
            .ca_params
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Root CA params not available"))?;
        let ca_key = self
            .ca_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Root CA key not available"))?;

        let mut params = CertificateParams::default();

        params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CountryName, &self.config.country);
        dn.push(DnType::StateOrProvinceName, &self.config.state);
        dn.push(DnType::LocalityName, &self.config.locality);
        dn.push(DnType::OrganizationName, &self.config.organization);
        dn.push(DnType::CommonName, "BotServer Intermediate CA");
        params.distinguished_name = dn;

        params.not_before = OffsetDateTime::now_utc();
        params.not_after = OffsetDateTime::now_utc() + Duration::days(self.config.validity_days);

        let key_pair = KeyPair::generate()?;

        let issuer = Issuer::from_params(ca_params, ca_key);

        let cert = params.signed_by(&key_pair, &issuer)?;

        if let (Some(cert_path), Some(key_path)) = (
            &self.config.intermediate_cert_path,
            &self.config.intermediate_key_path,
        ) {
            fs::write(cert_path, cert.pem())?;
            fs::write(key_path, key_pair.serialize_pem())?;
        }

        self.intermediate_params = Some(params);
        self.intermediate_key = Some(key_pair);

        info!("Generated intermediate CA certificate");
        Ok(())
    }

    pub fn issue_certificate(
        &self,
        common_name: &str,
        san_names: Vec<String>,
        is_client: bool,
    ) -> Result<(String, String)> {
        let (signing_params, signing_key) =
            match (&self.intermediate_params, &self.intermediate_key) {
                (Some(params), Some(key)) => (params, key),
                _ => match (&self.ca_params, &self.ca_key) {
                    (Some(params), Some(key)) => (params, key),
                    _ => return Err(anyhow::anyhow!("CA not initialized")),
                },
            };

        let mut params = CertificateParams::default();

        let mut dn = DistinguishedName::new();
        dn.push(DnType::CountryName, &self.config.country);
        dn.push(DnType::StateOrProvinceName, &self.config.state);
        dn.push(DnType::LocalityName, &self.config.locality);
        dn.push(DnType::OrganizationName, &self.config.organization);
        dn.push(DnType::CommonName, common_name);
        params.distinguished_name = dn;

        for san in san_names {
            if san.parse::<std::net::IpAddr>().is_ok() {
                params
                    .subject_alt_names
                    .push(SanType::IpAddress(san.parse()?));
            } else {
                params
                    .subject_alt_names
                    .push(SanType::DnsName(san.try_into()?));
            }
        }

        params.not_before = OffsetDateTime::now_utc();
        params.not_after = OffsetDateTime::now_utc() + Duration::days(self.config.validity_days);

        if is_client {
            params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ClientAuth];
        } else {
            params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
        }

        let key_pair = KeyPair::generate()?;

        let issuer = Issuer::from_params(signing_params, signing_key);

        let cert = params.signed_by(&key_pair, &issuer)?;
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();

        Ok((cert_pem, key_pem))
    }

    pub fn issue_service_certificates(&self) -> Result<()> {
        let services = vec![
            ("api", vec!["localhost", "api", "127.0.0.1"]),
            ("llm", vec!["localhost", "llm", "127.0.0.1"]),
            ("embedding", vec!["localhost", "embedding", "127.0.0.1"]),
            ("vectordb", vec!["localhost", "vectordb", "127.0.0.1"]),
            ("tables", vec!["localhost", "tables", "127.0.0.1"]),
            ("cache", vec!["localhost", "cache", "127.0.0.1"]),
            ("drive", vec!["localhost", "drive", "127.0.0.1"]),
            ("directory", vec!["localhost", "directory", "127.0.0.1"]),
            ("email", vec!["localhost", "email", "127.0.0.1"]),
            ("meet", vec!["localhost", "meet", "127.0.0.1"]),
        ];

        for (service, sans) in services {
            self.issue_service_certificate(service, sans)?;
        }

        Ok(())
    }

    pub fn issue_service_certificate(
        &self,
        service_name: &str,
        san_names: Vec<&str>,
    ) -> Result<()> {
        let cert_dir = PathBuf::from(format!("certs/{}", service_name));
        fs::create_dir_all(&cert_dir)?;

        let (cert_pem, key_pem) = self.issue_certificate(
            &format!("{}.botserver.local", service_name),
            san_names.iter().map(|s| (*s).to_string()).collect(),
            false,
        )?;

        fs::write(cert_dir.join("server.crt"), cert_pem)?;
        fs::write(cert_dir.join("server.key"), key_pem)?;

        let (client_cert_pem, client_key_pem) = self.issue_certificate(
            &format!("{}-client.botserver.local", service_name),
            vec![format!("{}-client", service_name)],
            true,
        )?;

        fs::write(cert_dir.join("client.crt"), client_cert_pem)?;
        fs::write(cert_dir.join("client.key"), client_key_pem)?;

        if let Ok(ca_cert) = fs::read_to_string(&self.config.ca_cert_path) {
            fs::write(cert_dir.join("ca.crt"), ca_cert)?;
        }

        info!("Issued certificates for service: {}", service_name);
        Ok(())
    }

    pub(super) fn create_ca_directories(&self) -> Result<()> {
        let ca_dir = self
            .config
            .ca_cert_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid CA cert path"))?;

        fs::create_dir_all(ca_dir)?;
        fs::create_dir_all("certs/api")?;
        fs::create_dir_all("certs/llm")?;
        fs::create_dir_all("certs/embedding")?;
        fs::create_dir_all("certs/vectordb")?;
        fs::create_dir_all("certs/tables")?;
        fs::create_dir_all("certs/cache")?;
        fs::create_dir_all("certs/drive")?;
        fs::create_dir_all("certs/directory")?;
        fs::create_dir_all("certs/email")?;
        fs::create_dir_all("certs/meet")?;

        Ok(())
    }
}
