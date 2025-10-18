use anyhow::{Context, Result};
use log::{debug, info, trace, warn};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum InstallMode {
    Local,
    Container,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OsType {
    Linux,
    MacOS,
    Windows,
}

#[derive(Debug, Clone)]
pub struct ComponentConfig {
    pub name: String,
    pub required: bool,
    pub ports: Vec<u16>,
    pub dependencies: Vec<String>,
    pub linux_packages: Vec<String>,
    pub macos_packages: Vec<String>,
    pub windows_packages: Vec<String>,
    pub download_url: Option<String>,
    pub binary_name: Option<String>,
    pub pre_install_cmds_linux: Vec<String>,
    pub post_install_cmds_linux: Vec<String>,
    pub pre_install_cmds_macos: Vec<String>,
    pub post_install_cmds_macos: Vec<String>,
    pub pre_install_cmds_windows: Vec<String>,
    pub post_install_cmds_windows: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub exec_cmd: String,
}

pub struct PackageManager {
    mode: InstallMode,
    os_type: OsType,
    base_path: PathBuf,
    tenant: String,
    components: HashMap<String, ComponentConfig>,
}

impl PackageManager {
    pub fn new(mode: InstallMode, tenant: Option<String>) -> Result<Self> {
        let os_type = Self::detect_os();
        let base_path = if mode == InstallMode::Container {
            PathBuf::from("/opt/gbo")
        } else {
            PathBuf::from("./botserver-stack")
        };
        
        let tenant = tenant.unwrap_or_else(|| "default".to_string());
        
        let mut pm = PackageManager {
            mode,
            os_type,
            base_path,
            tenant,
            components: HashMap::new(),
        };
        
        pm.register_components();
        info!("PackageManager initialized with {} components in {:?} mode for tenant {}", pm.components.len(), pm.mode, pm.tenant);
        Ok(pm)
    }

    fn detect_os() -> OsType {
        if cfg!(target_os = "linux") {
            OsType::Linux
        } else if cfg!(target_os = "macos") {
            OsType::MacOS
        } else if cfg!(target_os = "windows") {
            OsType::Windows
        } else {
            OsType::Linux
        }
    }

    fn register_components(&mut self) {
        self.register_drive();
        self.register_cache();
        self.register_tables();
        self.register_llm();
        self.register_email();
        self.register_proxy();
        self.register_directory();
        self.register_alm();
        self.register_alm_ci();
        self.register_dns();
        self.register_webmail();
        self.register_meeting();
        self.register_table_editor();
        self.register_doc_editor();
        self.register_desktop();
        self.register_devtools();
        self.register_bot();
        self.register_system();
        self.register_vector_db();
        self.register_host();
    }

    fn register_drive(&mut self) {
        self.components.insert("drive".to_string(), ComponentConfig {
            name: "drive".to_string(),
            required: true,
            ports: vec![9000, 9001],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string()],
            macos_packages: vec!["wget".to_string()],
            windows_packages: vec![],
            download_url: Some("https://dl.min.io/server/minio/release/linux-amd64/minio".to_string()),
            binary_name: Some("minio".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![
                "wget https://dl.min.io/client/mc/release/linux-amd64/mc -O {{BIN_PATH}}/mc".to_string(),
                "chmod +x {{BIN_PATH}}/mc".to_string()
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![
                "wget https://dl.min.io/client/mc/release/darwin-amd64/mc -O {{BIN_PATH}}/mc".to_string(),
                "chmod +x {{BIN_PATH}}/mc".to_string()
            ],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::from([
                ("MINIO_ROOT_USER".to_string(), "minioadmin".to_string()),
                ("MINIO_ROOT_PASSWORD".to_string(), "minioadmin".to_string())
            ]),
            exec_cmd: "{{BIN_PATH}}/minio server {{DATA_PATH}} --address :9000 --console-address :9001".to_string(),
        });
    }

    fn register_cache(&mut self) {
        self.components.insert("cache".to_string(), ComponentConfig {
            name: "cache".to_string(),
            required: true,
            ports: vec![6379],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "curl".to_string(), "gnupg".to_string(), "lsb-release".to_string()],
            macos_packages: vec!["redis".to_string()],
            windows_packages: vec![],
            download_url: None,
            binary_name: Some("valkey-server".to_string()),
            pre_install_cmds_linux: vec![
                "curl -fsSL https://packages.redis.io/gpg | gpg --dearmor -o /usr/share/keyrings/valkey.gpg".to_string(),
                "echo 'deb [signed-by=/usr/share/keyrings/valkey.gpg] https://packages.redis.io/deb $(lsb_release -cs) main' | tee /etc/apt/sources.list.d/valkey.list".to_string(),
                "apt-get update && apt-get install -y valkey".to_string()
            ],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "valkey-server --port 6379 --dir {{DATA_PATH}}".to_string(),
        });
    }

    fn register_tables(&mut self) {
        self.components.insert("tables".to_string(), ComponentConfig {
            name: "tables".to_string(),
            required: true,
            ports: vec![5432],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "gnupg2".to_string(), "lsb-release".to_string(), "postgresql-common".to_string()],
            macos_packages: vec!["postgresql".to_string()],
            windows_packages: vec![],
            download_url: None,
            binary_name: Some("postgres".to_string()),
            pre_install_cmds_linux: vec![
                "/usr/share/postgresql-common/pgdg/apt.postgresql.org.sh".to_string(),
                "apt-get update && apt-get install -y postgresql-16".to_string()
            ],
            post_install_cmds_linux: vec![
                "sudo -u postgres psql -p 5432 -c \"CREATE USER default WITH PASSWORD 'defaultpass'\" || true".to_string(),
                "sudo -u postgres psql -p 5432 -c \"CREATE DATABASE default_db OWNER default\" || true".to_string(),
                "sudo -u postgres psql -p 5432 -c \"GRANT ALL PRIVILEGES ON DATABASE default_db TO default\" || true".to_string()
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![
                "initdb -D {{DATA_PATH}}/pgdata".to_string(),
                "sleep 5".to_string(),
                "psql -p 5432 -d postgres -c \"CREATE USER default WITH PASSWORD 'defaultpass'\" || true".to_string(),
                "psql -p 5432 -d postgres -c \"CREATE DATABASE default_db OWNER default\" || true".to_string()
            ],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "postgres -D {{DATA_PATH}}/pgdata -p 5432".to_string(),
        });
    }

    fn register_llm(&mut self) {
        self.components.insert("llm".to_string(), ComponentConfig {
            name: "llm".to_string(),
            required: true,
            ports: vec![8081],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "unzip".to_string()],
            macos_packages: vec!["wget".to_string(), "unzip".to_string()],
            windows_packages: vec![],
            download_url: Some("https://github.com/ggml-org/llama.cpp/releases/download/b6148/llama-b6148-bin-ubuntu-x64.zip".to_string()),
            binary_name: Some("llama-server".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![
                "wget https://huggingface.co/bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF/resolve/main/DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf -P {{DATA_PATH}}".to_string(),
                "wget https://huggingface.co/CompendiumLabs/bge-small-en-v1.5-gguf/resolve/main/bge-small-en-v1.5-f32.gguf -P {{DATA_PATH}}".to_string()
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![
                "wget https://huggingface.co/bartowski/DeepSeek-R1-Distill-Qwen-1.5B-GGUF/resolve/main/DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf -P {{DATA_PATH}}".to_string(),
                "wget https://huggingface.co/CompendiumLabs/bge-small-en-v1.5-gguf/resolve/main/bge-small-en-v1.5-f32.gguf -P {{DATA_PATH}}".to_string()
            ],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "{{BIN_PATH}}/llama-server -m {{DATA_PATH}}/DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf --port 8081".to_string(),
        });
    }

    fn register_email(&mut self) {
        self.components.insert("email".to_string(), ComponentConfig {
            name: "email".to_string(),
            required: false,
            ports: vec![25, 80, 110, 143, 465, 587, 993, 995, 4190],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "libcap2-bin".to_string(), "resolvconf".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: Some("https://github.com/stalwartlabs/stalwart/releases/download/v0.13.1/stalwart-x86_64-unknown-linux-gnu.tar.gz".to_string()),
            binary_name: Some("stalwart".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![
                "setcap 'cap_net_bind_service=+ep' {{BIN_PATH}}/stalwart".to_string()
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "{{BIN_PATH}}/stalwart --config {{CONF_PATH}}/config.toml".to_string(),
        });
    }

    fn register_proxy(&mut self) {
        self.components.insert("proxy".to_string(), ComponentConfig {
            name: "proxy".to_string(),
            required: false,
            ports: vec![80, 443],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "libcap2-bin".to_string()],
            macos_packages: vec!["wget".to_string()],
            windows_packages: vec![],
            download_url: Some("https://github.com/caddyserver/caddy/releases/download/v2.10.0-beta.3/caddy_2.10.0-beta.3_linux_amd64.tar.gz".to_string()),
            binary_name: Some("caddy".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![
                "setcap 'cap_net_bind_service=+ep' {{BIN_PATH}}/caddy".to_string()
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::from([
                ("XDG_DATA_HOME".to_string(), "{{DATA_PATH}}".to_string())
            ]),
            exec_cmd: "{{BIN_PATH}}/caddy run --config {{CONF_PATH}}/Caddyfile".to_string(),
        });
    }

    fn register_directory(&mut self) {
        self.components.insert("directory".to_string(), ComponentConfig {
            name: "directory".to_string(),
            required: false,
            ports: vec![8080],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "libcap2-bin".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: Some("https://github.com/zitadel/zitadel/releases/download/v2.71.2/zitadel-linux-amd64.tar.gz".to_string()),
            binary_name: Some("zitadel".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![
                "setcap 'cap_net_bind_service=+ep' {{BIN_PATH}}/zitadel".to_string()
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "{{BIN_PATH}}/zitadel start --config {{CONF_PATH}}/zitadel.yaml".to_string(),
        });
    }

    fn register_alm(&mut self) {
        self.components.insert("alm".to_string(), ComponentConfig {
            name: "alm".to_string(),
            required: false,
            ports: vec![3000],
            dependencies: vec![],
            linux_packages: vec!["git".to_string(), "git-lfs".to_string(), "wget".to_string()],
            macos_packages: vec!["git".to_string(), "git-lfs".to_string()],
            windows_packages: vec![],
            download_url: Some("https://codeberg.org/forgejo/forgejo/releases/download/v10.0.2/forgejo-10.0.2-linux-amd64".to_string()),
            binary_name: Some("forgejo".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::from([
                ("USER".to_string(), "alm".to_string()),
                ("HOME".to_string(), "{{DATA_PATH}}".to_string())
            ]),
            exec_cmd: "{{BIN_PATH}}/forgejo web --work-path {{DATA_PATH}}".to_string(),
        });
    }

    fn register_alm_ci(&mut self) {
        self.components.insert("alm-ci".to_string(), ComponentConfig {
            name: "alm-ci".to_string(),
            required: false,
            ports: vec![],
            dependencies: vec!["alm".to_string()],
            linux_packages: vec!["wget".to_string(), "git".to_string(), "curl".to_string(), "gnupg".to_string(), "ca-certificates".to_string(), "build-essential".to_string()],
            macos_packages: vec!["git".to_string(), "node".to_string()],
            windows_packages: vec![],
            download_url: Some("https://code.forgejo.org/forgejo/runner/releases/download/v6.3.1/forgejo-runner-6.3.1-linux-amd64".to_string()),
            binary_name: Some("forgejo-runner".to_string()),
            pre_install_cmds_linux: vec![
                "curl -fsSL https://deb.nodesource.com/setup_22.x | bash -".to_string(),
                "apt-get update && apt-get install -y nodejs".to_string()
            ],
            post_install_cmds_linux: vec![
                "npm install -g pnpm@latest".to_string()
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![
                "npm install -g pnpm@latest".to_string()
            ],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "{{BIN_PATH}}/forgejo-runner daemon --config {{CONF_PATH}}/config.yaml".to_string(),
        });
    }

    fn register_dns(&mut self) {
        self.components.insert("dns".to_string(), ComponentConfig {
            name: "dns".to_string(),
            required: false,
            ports: vec![53],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: Some("https://github.com/coredns/coredns/releases/download/v1.12.4/coredns_1.12.4_linux_amd64.tgz".to_string()),
            binary_name: Some("coredns".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![
                "setcap cap_net_bind_service=+ep {{BIN_PATH}}/coredns".to_string()
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "{{BIN_PATH}}/coredns -conf {{CONF_PATH}}/Corefile".to_string(),
        });
    }

    fn register_webmail(&mut self) {
        self.components.insert("webmail".to_string(), ComponentConfig {
            name: "webmail".to_string(),
            required: false,
            ports: vec![8080],
            dependencies: vec!["email".to_string()],
            linux_packages: vec!["ca-certificates".to_string(), "apt-transport-https".to_string(), "php8.1".to_string(), "php8.1-fpm".to_string()],
            macos_packages: vec!["php".to_string()],
            windows_packages: vec![],
            download_url: Some("https://github.com/roundcube/roundcubemail/releases/download/1.6.6/roundcubemail-1.6.6-complete.tar.gz".to_string()),
            binary_name: None,
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "php -S 0.0.0.0:8080 -t {{DATA_PATH}}/roundcubemail".to_string(),
        });
    }

    fn register_meeting(&mut self) {
        self.components.insert("meeting".to_string(), ComponentConfig {
            name: "meeting".to_string(),
            required: false,
            ports: vec![7880, 3478],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "coturn".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: Some("https://github.com/livekit/livekit/releases/download/v1.8.4/livekit_1.8.4_linux_amd64.tar.gz".to_string()),
            binary_name: Some("livekit-server".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "{{BIN_PATH}}/livekit-server --config {{CONF_PATH}}/config.yaml".to_string(),
        });
    }

    fn register_table_editor(&mut self) {
        self.components.insert("table-editor".to_string(), ComponentConfig {
            name: "table-editor".to_string(),
            required: false,
            ports: vec![5757],
            dependencies: vec!["tables".to_string()],
            linux_packages: vec!["wget".to_string(), "curl".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: Some("http://get.nocodb.com/linux-x64".to_string()),
            binary_name: Some("nocodb".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "{{BIN_PATH}}/nocodb".to_string(),
        });
    }

    fn register_doc_editor(&mut self) {
        self.components.insert("doc-editor".to_string(), ComponentConfig {
            name: "doc-editor".to_string(),
            required: false,
            ports: vec![9980],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "gnupg".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: None,
            binary_name: Some("coolwsd".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "coolwsd --config-file={{CONF_PATH}}/coolwsd.xml".to_string(),
        });
    }

    fn register_desktop(&mut self) {
        self.components.insert("desktop".to_string(), ComponentConfig {
            name: "desktop".to_string(),
            required: false,
            ports: vec![3389],
            dependencies: vec![],
            linux_packages: vec!["xvfb".to_string(), "xrdp".to_string(), "xfce4".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: None,
            binary_name: None,
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "xrdp --nodaemon".to_string(),
        });
    }

    fn register_devtools(&mut self) {
        self.components.insert("devtools".to_string(), ComponentConfig {
            name: "devtools".to_string(),
            required: false,
            ports: vec![],
            dependencies: vec![],
            linux_packages: vec!["xclip".to_string(), "git".to_string(), "curl".to_string()],
            macos_packages: vec!["git".to_string()],
            windows_packages: vec![],
            download_url: None,
            binary_name: None,
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "".to_string(),
        });
    }

    fn register_bot(&mut self) {
        self.components.insert("bot".to_string(), ComponentConfig {
            name: "bot".to_string(),
            required: false,
            ports: vec![3000],
            dependencies: vec![],
            linux_packages: vec!["curl".to_string(), "gnupg".to_string(), "ca-certificates".to_string(), "git".to_string()],
            macos_packages: vec!["node".to_string()],
            windows_packages: vec![],
            download_url: None,
            binary_name: None,
            pre_install_cmds_linux: vec![
                "curl -fsSL https://deb.nodesource.com/setup_22.x | bash -".to_string(),
                "apt-get update && apt-get install -y nodejs".to_string()
            ],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::from([
                ("DISPLAY".to_string(), ":99".to_string())
            ]),
            exec_cmd: "".to_string(),
        });
    }

    fn register_system(&mut self) {
        self.components.insert("system".to_string(), ComponentConfig {
            name: "system".to_string(),
            required: false,
            ports: vec![8000],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string(), "curl".to_string(), "unzip".to_string(), "git".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: None,
            binary_name: None,
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "".to_string(),
        });
    }

    fn register_vector_db(&mut self) {
        self.components.insert("vector-db".to_string(), ComponentConfig {
            name: "vector-db".to_string(),
            required: false,
            ports: vec![6333],
            dependencies: vec![],
            linux_packages: vec!["wget".to_string()],
            macos_packages: vec!["wget".to_string()],
            windows_packages: vec![],
            download_url: Some("https://github.com/qdrant/qdrant/releases/latest/download/qdrant-x86_64-unknown-linux-gnu.tar.gz".to_string()),
            binary_name: Some("qdrant".to_string()),
            pre_install_cmds_linux: vec![],
            post_install_cmds_linux: vec![],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "{{BIN_PATH}}/qdrant --storage-path {{DATA_PATH}}".to_string(),
        });
    }

    fn register_host(&mut self) {
        self.components.insert("host".to_string(), ComponentConfig {
            name: "host".to_string(),
            required: false,
            ports: vec![],
            dependencies: vec![],
            linux_packages: vec!["sshfs".to_string(), "bridge-utils".to_string()],
            macos_packages: vec![],
            windows_packages: vec![],
            download_url: None,
            binary_name: None,
            pre_install_cmds_linux: vec![
                "echo 'net.ipv4.ip_forward=1' | tee -a /etc/sysctl.conf".to_string(),
                "sysctl -p".to_string(),
            ],
            post_install_cmds_linux: vec![
                "lxd init --auto".to_string(),
                "lxc storage create default dir".to_string(),
                "lxc profile device add default root disk path=/ pool=default".to_string(),
            ],
            pre_install_cmds_macos: vec![],
            post_install_cmds_macos: vec![],
            pre_install_cmds_windows: vec![],
            post_install_cmds_windows: vec![],
            env_vars: HashMap::new(),
            exec_cmd: "".to_string(),
        });
    }

    pub fn install(&self, component_name: &str) -> Result<()> {
        let component = self.components.get(component_name).context(format!("Component '{}' not found", component_name))?;
        
        info!("Starting installation of component '{}' in {:?} mode", component_name, self.mode);
        
        for dep in &component.dependencies {
            if !self.is_installed(dep) {
                warn!("Installing missing dependency: {}", dep);
                self.install(dep)?;
            }
        }
        
        match self.mode {
            InstallMode::Local => self.install_local(component)?,
            InstallMode::Container => self.install_container(component)?,
        }
        
        info!("Component '{}' installation completed successfully", component_name);
        Ok(())
    }

    fn install_local(&self, component: &ComponentConfig) -> Result<()> {
        info!("Installing component '{}' locally to {}", component.name, self.base_path.display());
        
        self.create_directories(&component.name)?;
        
        let (pre_cmds, post_cmds) = match self.os_type {
            OsType::Linux => (&component.pre_install_cmds_linux, &component.post_install_cmds_linux),
            OsType::MacOS => (&component.pre_install_cmds_macos, &component.post_install_cmds_macos),
            OsType::Windows => (&component.pre_install_cmds_windows, &component.post_install_cmds_windows),
        };
        
        self.run_commands(pre_cmds, "local", &component.name)?;
        self.install_system_packages(component)?;
        
        if let Some(url) = &component.download_url {
            self.download_and_install(url, &component.name, component.binary_name.as_deref())?;
        }
        
        self.run_commands(post_cmds, "local", &component.name)?;
        
        if self.os_type == OsType::Linux && !component.exec_cmd.is_empty() {
            self.create_service_file(&component.name, &component.exec_cmd, &component.env_vars)?;
        }
        
        info!("Local installation of '{}' completed", component.name);
        Ok(())
    }

    fn install_container(&self, component: &ComponentConfig) -> Result<()> {
        let container_name = format!("{}-{}", self.tenant, component.name);
        info!("Creating LXC container: {}", container_name);
        
        let output = Command::new("lxc").args(&["launch", "images:debian/12", &container_name, "-c", "security.privileged=true"]).output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("LXC container creation failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
        
        std::thread::sleep(std::time::Duration::from_secs(15));
        
        self.exec_in_container(&container_name, "mkdir -p /opt/gbo/{bin,data,conf,logs}")?;
        
        let (pre_cmds, post_cmds) = match self.os_type {
            OsType::Linux => (&component.pre_install_cmds_linux, &component.post_install_cmds_linux),
            OsType::MacOS => (&component.pre_install_cmds_macos, &component.post_install_cmds_macos),
            OsType::Windows => (&component.pre_install_cmds_windows, &component.post_install_cmds_windows),
        };
        
        self.run_commands(pre_cmds, &container_name, &component.name)?;
        
        let packages = match self.os_type {
            OsType::Linux => &component.linux_packages,
            OsType::MacOS => &component.macos_packages,
            OsType::Windows => &component.windows_packages,
        };
        
        if !packages.is_empty() {
            let pkg_list = packages.join(" ");
            self.exec_in_container(&container_name, &format!("apt-get update && apt-get install -y {}", pkg_list))?;
        }
        
        if let Some(url) = &component.download_url {
            self.download_in_container(&container_name, url, &component.name, component.binary_name.as_deref())?;
        }
        
        self.run_commands(post_cmds, &container_name, &component.name)?;
        
        self.exec_in_container(&container_name, "useradd --system --no-create-home --shell /bin/false gbuser")?;
        self.mount_container_directories(&container_name, &component.name)?;
        
        if !component.exec_cmd.is_empty() {
            self.create_container_service(&container_name, &component.name, &component.exec_cmd, &component.env_vars)?;
        }
        
        self.setup_port_forwarding(&container_name, &component.ports)?;
        
        info!("Container installation of '{}' completed in {}", component.name, container_name);
        Ok(())
    }

    pub fn remove(&self, component_name: &str) -> Result<()> {
        let component = self.components.get(component_name).context(format!("Component '{}' not found", component_name))?;
        
        info!("Removing component: {}", component_name);
        
        match self.mode {
            InstallMode::Local => self.remove_local(component)?,
            InstallMode::Container => self.remove_container(component)?,
        }
        
        info!("Component '{}' removed successfully", component_name);
        Ok(())
    }

    fn remove_local(&self, component: &ComponentConfig) -> Result<()> {
        if self.os_type == OsType::Linux {
            let _ = Command::new("systemctl").args(&["stop", &format!("{}.service", component.name)]).output();
            let _ = Command::new("systemctl").args(&["disable", &format!("{}.service", component.name)]).output();
            let service_path = format!("/etc/systemd/system/{}.service", component.name);
            let _ = std::fs::remove_file(service_path);
            let _ = Command::new("systemctl").args(&["daemon-reload"]).output();
        }
        
        let bin_path = self.base_path.join("bin").join(&component.name);
        let data_path = self.base_path.join("data").join(&component.name);
        let conf_path = self.base_path.join("conf").join(&component.name);
        let logs_path = self.base_path.join("logs").join(&component.name);
        
        let _ = std::fs::remove_dir_all(bin_path);
        let _ = std::fs::remove_dir_all(data_path);
        let _ = std::fs::remove_dir_all(conf_path);
        let _ = std::fs::remove_dir_all(logs_path);
        
        Ok(())
    }

    fn remove_container(&self, component: &ComponentConfig) -> Result<()> {
        let container_name = format!("{}-{}", self.tenant, component.name);
        
        let _ = Command::new("lxc").args(&["stop", &container_name]).output();
        let output = Command::new("lxc").args(&["delete", &container_name]).output()?;
        
        if !output.status.success() {
            warn!("Container deletion had issues: {}", String::from_utf8_lossy(&output.stderr));
        }
        
        let host_base = format!("/opt/gbo/tenants/{}/{}", self.tenant, component.name);
        let _ = std::fs::remove_dir_all(host_base);
        
        Ok(())
    }

    pub fn list(&self) -> Vec<String> {
        self.components.keys().cloned().collect()
    }

    pub fn is_installed(&self, component_name: &str) -> bool {
        match self.mode {
            InstallMode::Local => {
                let bin_path = self.base_path.join("bin").join(component_name);
                bin_path.exists()
            }
            InstallMode::Container => {
                let container_name = format!("{}-{}", self.tenant, component_name);
                Command::new("lxc").args(&["list", &container_name, "--format=json"]).output().map(|o| o.status.success()).unwrap_or(false)
            }
        }
    }

    fn create_directories(&self, component: &str) -> Result<()> {
        let dirs = ["bin", "data", "conf", "logs"];
        for dir in &dirs {
            let path = self.base_path.join(dir).join(component);
            std::fs::create_dir_all(&path).context(format!("Failed to create directory: {:?}", path))?;
            trace!("Created directory: {:?}", path);
        }
        Ok(())
    }

    fn install_system_packages(&self, component: &ComponentConfig) -> Result<()> {
        let packages = match self.os_type {
            OsType::Linux => &component.linux_packages,
            OsType::MacOS => &component.macos_packages,
            OsType::Windows => &component.windows_packages,
        };
        
        if packages.is_empty() {
            return Ok(());
        }
        
        info!("Installing {} system packages for component '{}'", packages.len(), component.name);
        
        match self.os_type {
            OsType::Linux => {
                let output = Command::new("apt-get").args(&["install", "-y"]).args(packages).output()?;
                if !output.status.success() {
                    warn!("Some packages may have failed to install");
                }
            }
            OsType::MacOS => {
                let output = Command::new("brew").args(&["install"]).args(packages).output()?;
                if !output.status.success() {
                    warn!("Homebrew installation had warnings");
                }
            }
            OsType::Windows => {
                warn!("Windows package installation not implemented");
            }
        }
        
        Ok(())
    }

    fn download_and_install(&self, url: &str, component: &str, binary_name: Option<&str>) -> Result<()> {
        let bin_path = self.base_path.join("bin").join(component);
        let temp_file = bin_path.join("download.tmp");
        
        info!("Downloading from: {}", url);
        let output = Command::new("wget").args(&["-O", temp_file.to_str().unwrap(), url]).output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Download failed from URL: {}", url));
        }
        
        if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
            trace!("Extracting tar.gz archive to {:?}", bin_path);
            Command::new("tar").args(&["-xzf", temp_file.to_str().unwrap(), "-C", bin_path.to_str().unwrap()]).output()?;
            std::fs::remove_file(&temp_file)?;
        } else if url.ends_with(".zip") {
            trace!("Extracting zip archive to {:?}", bin_path);
            Command::new("unzip").args(&[temp_file.to_str().unwrap(), "-d", bin_path.to_str().unwrap()]).output()?;
            std::fs::remove_file(&temp_file)?;
        } else if let Some(name) = binary_name {
            let final_path = bin_path.join(name);
            std::fs::rename(&temp_file, &final_path)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&final_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&final_path, perms)?;
            }
        }
        
        Ok(())
    }

    fn create_service_file(&self, component: &str, exec_cmd: &str, env_vars: &HashMap<String, String>) -> Result<()> {
        let service_path = format!("/etc/systemd/system/{}.service", component);
        
        let bin_path = self.base_path.join("bin").join(component).to_string_lossy().to_string();
        let data_path = self.base_path.join("data").join(component).to_string_lossy().to_string();
        let conf_path = self.base_path.join("conf").join(component).to_string_lossy().to_string();
        let logs_path = self.base_path.join("logs").join(component).to_string_lossy().to_string();
        
        let rendered_cmd = exec_cmd
            .replace("{{BIN_PATH}}", &bin_path)
            .replace("{{DATA_PATH}}", &data_path)
            .replace("{{CONF_PATH}}", &conf_path)
            .replace("{{LOGS_PATH}}", &logs_path);
        
        let mut env_section = String::new();
        for (key, value) in env_vars {
            let rendered_value = value.replace("{{DATA_PATH}}", &data_path);
            env_section.push_str(&format!("Environment=\"{}={}\"\n", key, rendered_value));
        }
        
        let service_content = format!("[Unit]\nDescription={} Service\nAfter=network.target\n\n[Service]\nType=simple\n{}ExecStart={}\nWorkingDirectory={}\nRestart=always\nRestartSec=10\nUser=root\n\n[Install]\nWantedBy=multi-user.target\n", component, env_section, rendered_cmd, data_path);
        
        std::fs::write(&service_path, service_content)?;
        
        Command::new("systemctl").args(&["daemon-reload"]).output()?;
        Command::new("systemctl").args(&["enable", &format!("{}.service", component)]).output()?;
        Command::new("systemctl").args(&["start", &format!("{}.service", component)]).output()?;
        
        info!("Created and started systemd service: {}.service", component);
        Ok(())
    }

    fn run_commands(&self, commands: &[String], target: &str, component: &str) -> Result<()> {
        for cmd in commands {
            let bin_path = if target == "local" {
                self.base_path.join("bin").join(component).to_string_lossy().to_string()
            } else {
                "/opt/gbo/bin".to_string()
            };
            
            let data_path = if target == "local" {
                self.base_path.join("data").join(component).to_string_lossy().to_string()
            } else {
                "/opt/gbo/data".to_string()
            };
            
            let conf_path = if target == "local" {
                self.base_path.join("conf").join(component).to_string_lossy().to_string()
            } else {
                "/opt/gbo/conf".to_string()
            };
            
            let logs_path = if target == "local" {
                self.base_path.join("logs").join(component).to_string_lossy().to_string()
            } else {
                "/opt/gbo/logs".to_string()
            };
            
            let rendered_cmd = cmd
                .replace("{{BIN_PATH}}", &bin_path)
                .replace("{{DATA_PATH}}", &data_path)
                .replace("{{CONF_PATH}}", &conf_path)
                .replace("{{LOGS_PATH}}", &logs_path);
            
            trace!("Executing command: {}", rendered_cmd);
            
            if target == "local" {
                let output = Command::new("bash").args(&["-c", &rendered_cmd]).output()?;
                if !output.status.success() {
                    warn!("Command had non-zero exit: {}", String::from_utf8_lossy(&output.stderr));
                }
            } else {
                self.exec_in_container(target, &rendered_cmd)?;
            }
        }
        Ok(())
    }

    fn exec_in_container(&self, container: &str, command: &str) -> Result<()> {
        debug!("Executing in container {}: {}", container, command);
        let output = Command::new("lxc").args(&["exec", container, "--", "bash", "-c", command]).output()?;
        
        if !output.status.success() {
            warn!("Container command failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    fn download_in_container(&self, container: &str, url: &str, _component: &str, binary_name: Option<&str>) -> Result<()> {
        let download_cmd = format!("wget -O /tmp/download.tmp {}", url);
        self.exec_in_container(container, &download_cmd)?;
        
        if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
            self.exec_in_container(container, "tar -xzf /tmp/download.tmp -C /opt/gbo/bin")?;
        } else if url.ends_with(".zip") {
            self.exec_in_container(container, "unzip /tmp/download.tmp -d /opt/gbo/bin")?;
        } else if let Some(name) = binary_name {
            let mv_cmd = format!("mv /tmp/download.tmp /opt/gbo/bin/{} && chmod +x /opt/gbo/bin/{}", name, name);
            self.exec_in_container(container, &mv_cmd)?;
        }
        
        self.exec_in_container(container, "rm -f /tmp/download.tmp")?;
        Ok(())
    }

    fn mount_container_directories(&self, container: &str, component: &str) -> Result<()> {
        let host_base = format!("/opt/gbo/tenants/{}/{}", self.tenant, component);
        
        for dir in &["data", "conf", "logs"] {
            let host_path = format!("{}/{}", host_base, dir);
            std::fs::create_dir_all(&host_path)?;
            
            let device_name = format!("{}{}", component, dir);
            let container_path = format!("/opt/gbo/{}", dir);
            
            let _ = Command::new("lxc").args(&["config", "device", "remove", container, &device_name]).output();
            
            Command::new("lxc").args(&["config", "device", "add", container, &device_name, "disk", &format!("source={}", host_path), &format!("path={}", container_path)]).output()?;
            
            trace!("Mounted {} to {} in container {}", host_path, container_path, container);
        }
        Ok(())
    }

    fn create_container_service(&self, container: &str, component: &str, exec_cmd: &str, env_vars: &HashMap<String, String>) -> Result<()> {
        let rendered_cmd = exec_cmd
            .replace("{{BIN_PATH}}", "/opt/gbo/bin")
            .replace("{{DATA_PATH}}", "/opt/gbo/data")
            .replace("{{CONF_PATH}}", "/opt/gbo/conf")
            .replace("{{LOGS_PATH}}", "/opt/gbo/logs");
        
        let mut env_section = String::new();
        for (key, value) in env_vars {
            let rendered_value = value.replace("{{DATA_PATH}}", "/opt/gbo/data");
            env_section.push_str(&format!("Environment=\"{}={}\"\n", key, rendered_value));
        }
        
        let service_content = format!("[Unit]\nDescription={} Service\nAfter=network.target\n\n[Service]\nType=simple\n{}ExecStart={}\nWorkingDirectory=/opt/gbo/data\nRestart=always\nRestartSec=10\nUser=root\n\n[Install]\nWantedBy=multi-user.target\n", component, env_section, rendered_cmd);
        
        let service_file = format!("/tmp/{}.service", component);
        std::fs::write(&service_file, &service_content)?;
        
        Command::new("lxc").args(&["file", "push", &service_file, &format!("{}/etc/systemd/system/{}.service", container, component)]).output()?;
        
        self.exec_in_container(container, "systemctl daemon-reload")?;
        self.exec_in_container(container, &format!("systemctl enable {}", component))?;
        self.exec_in_container(container, &format!("systemctl start {}", component))?;
        
        std::fs::remove_file(&service_file)?;
        
        info!("Created and started service in container {}: {}", container, component);
        Ok(())
    }

    fn setup_port_forwarding(&self, container: &str, ports: &[u16]) -> Result<()> {
        for port in ports {
            let device_name = format!("port-{}", port);
            
            let _ = Command::new("lxc").args(&["config", "device", "remove", container, &device_name]).output();
            
            Command::new("lxc").args(&["config", "device", "add", container, &device_name, "proxy", &format!("listen=tcp:0.0.0.0:{}", port), &format!("connect=tcp:127.0.0.1:{}", port)]).output()?;
            
            trace!("Port forwarding configured: {} -> container {}", port, container);
        }
        Ok(())
    }
}

pub mod cli {
    use super::*;
    use std::env;

    pub fn run() -> Result<()> {
        env_logger::init();
        let args: Vec<String> = env::args().collect();
        
        if args.len() < 2 {
            print_usage();
            return Ok(());
        }
        
        let command = &args[1];
        
        match command.as_str() {
            "install" => {
                if args.len() < 3 {
                    eprintln!("Usage: botserver install <component> [--container] [--tenant <name>]");
                    return Ok(());
                }
                
                let component = &args[2];
                let mode = if args.contains(&"--container".to_string()) { InstallMode::Container } else { InstallMode::Local };
                let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") { args.get(idx + 1).cloned() } else { None };
                
                let pm = PackageManager::new(mode, tenant)?;
                pm.install(component)?;
                println!("✓ Component '{}' installed successfully", component);
            }
            "remove" => {
                if args.len() < 3 {
                    eprintln!("Usage: botserver remove <component> [--container] [--tenant <name>]");
                    return Ok(());
                }
                
                let component = &args[2];
                let mode = if args.contains(&"--container".to_string()) { InstallMode::Container } else { InstallMode::Local };
                let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") { args.get(idx + 1).cloned() } else { None };
                
                let pm = PackageManager::new(mode, tenant)?;
                pm.remove(component)?;
                println!("✓ Component '{}' removed successfully", component);
            }
            "list" => {
                let mode = if args.contains(&"--container".to_string()) { InstallMode::Container } else { InstallMode::Local };
                let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") { args.get(idx + 1).cloned() } else { None };
                
                let pm = PackageManager::new(mode, tenant)?;
                println!("Available components:");
                for component in pm.list() {
                    let status = if pm.is_installed(&component) { "✓ installed" } else { "  available" };
                    println!(" {} {}", status, component);
                }
            }
            "status" => {
                if args.len() < 3 {
                    eprintln!("Usage: botserver status <component> [--container] [--tenant <name>]");
                    return Ok(());
                }
                
                let component = &args[2];
                let mode = if args.contains(&"--container".to_string()) { InstallMode::Container } else { InstallMode::Local };
                let tenant = if let Some(idx) = args.iter().position(|a| a == "--tenant") { args.get(idx + 1).cloned() } else { None };
                
                let pm = PackageManager::new(mode, tenant)?;
                if pm.is_installed(component) {
                    println!("✓ Component '{}' is installed", component);
                } else {
                    println!("✗ Component '{}' is not installed", component);
                }
            }
            "--help" | "-h" => {
                print_usage();
            }
            _ => {
                eprintln!("Unknown command: {}", command);
                print_usage();
            }
        }
        
        Ok(())
    }

    fn print_usage() {
        println!("BotServer Package Manager\n\nUSAGE:\n  botserver <command> [options]\n\nCOMMANDS:\n  install <component>    Install component\n  remove <component>     Remove component\n  list                   List all components\n  status <component>     Check component status\n\nOPTIONS:\n  --container            Use container mode (LXC)\n  --tenant <name>        Specify tenant (default: 'default')\n\nCOMPONENTS:\n  Required: drive cache tables llm\n  Optional: email proxy directory alm alm-ci dns webmail meeting table-editor doc-editor desktop devtools bot system vector-db host\n\nEXAMPLES:\n  botserver install email\n  botserver install email --container --tenant myorg\n  botserver remove email\n  botserver list");
    }
}
