use super::*;

impl McpCsvLoader {
    pub fn new(work_path: &str, bot_id: &str) -> Self {
        Self {
            work_path: work_path.to_string(),
            bot_id: bot_id.to_string(),
        }
    }

    pub fn get_csv_path(&self) -> PathBuf {
        PathBuf::from(&self.work_path)
            .join(format!("{}.gbai", self.bot_id))
            .join("mcp.csv")
    }

    pub fn csv_exists(&self) -> bool {
        self.get_csv_path().exists()
    }

    pub fn load(&self) -> McpLoadResult {
        let csv_path = self.get_csv_path();

        info!("Loading MCP servers from: {}", csv_path.display());

        let mut result = McpLoadResult {
            servers: Vec::new(),
            errors: Vec::new(),
            lines_processed: 0,
            file_path: csv_path.clone(),
        };

        if !csv_path.exists() {
            debug!("MCP CSV file does not exist: {}", csv_path.display());
            return result;
        }

        let content = match std::fs::read_to_string(&csv_path) {
            Ok(c) => c,
            Err(e) => {
                result.errors.push(McpLoadError {
                    line: 0,
                    message: format!("Failed to read mcp.csv: {}", e),
                    recoverable: false,
                });
                return result;
            }
        };

        let mut lines = content.lines().enumerate();

        if let Some((_, header)) = lines.next() {
            let header_lower = header.to_lowercase();
            if !header_lower.starts_with("name,") && !header_lower.contains(",type,") {
                if let Some(server) = self.parse_csv_line(1, header, &mut result.errors) {
                    result.servers.push(server);
                }
            }
            result.lines_processed += 1;
        }

        for (line_num, line) in lines {
            let line_number = line_num + 1;
            result.lines_processed += 1;

            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            if let Some(server) = self.parse_csv_line(line_number, line, &mut result.errors) {
                result.servers.push(server);
            }
        }

        info!(
            "MCP load complete: {} servers loaded, {} errors, {} lines processed",
            result.servers.len(),
            result.errors.len(),
            result.lines_processed
        );

        result
    }

    pub(super) fn parse_csv_line(
        &self,
        line_num: usize,
        line: &str,
        errors: &mut Vec<McpLoadError>,
    ) -> Option<McpServer> {
        let columns = Self::parse_csv_columns(line);

        if columns.len() < 3 {
            errors.push(McpLoadError {
                line: line_num,
                message: format!(
                    "Invalid CSV: expected at least 3 columns (name,type,command), got {}",
                    columns.len()
                ),
                recoverable: true,
            });
            return None;
        }

        let name = columns[0].trim().to_string();
        let conn_type = columns[1].trim().to_lowercase();
        let command = columns[2].trim().to_string();
        let args = columns
            .get(3)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let description = columns
            .get(4)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let enabled = columns
            .get(5)
            .map(|s| s.trim().to_lowercase() != "false")
            .unwrap_or(true);
        let auth_type = columns
            .get(6)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let auth_env = columns
            .get(7)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let _risk_level = columns
            .get(8)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let _requires_approval = columns
            .get(9)
            .map(|s| s.trim().to_lowercase() == "true")
            .unwrap_or(false);

        if name.is_empty() {
            errors.push(McpLoadError {
                line: line_num,
                message: "Server name cannot be empty".to_string(),
                recoverable: true,
            });
            return None;
        }

        if command.is_empty() {
            errors.push(McpLoadError {
                line: line_num,
                message: format!("Command/URL cannot be empty for server '{}'", name),
                recoverable: true,
            });
            return None;
        }

        let connection = match conn_type.as_str() {
            "stdio" => {
                let args_vec: Vec<String> = if args.is_empty() {
                    Vec::new()
                } else {
                    Self::parse_args(&args)
                };
                McpConnection {
                    connection_type: ConnectionType::Stdio,
                    url: format!("{}:{}", command, args_vec.join(" ")),
                    port: None,
                    timeout_seconds: 30,
                    max_retries: 3,
                    retry_backoff_ms: 1000,
                    keep_alive: true,
                    tls_config: None,
                }
            }
            "http" => McpConnection {
                connection_type: ConnectionType::Http,
                url: command.clone(),
                port: None,
                timeout_seconds: 30,
                max_retries: 3,
                retry_backoff_ms: 1000,
                keep_alive: true,
                tls_config: None,
            },
            "websocket" | "ws" => McpConnection {
                connection_type: ConnectionType::WebSocket,
                url: command.clone(),
                port: None,
                timeout_seconds: 30,
                max_retries: 3,
                retry_backoff_ms: 1000,
                keep_alive: true,
                tls_config: None,
            },
            "tcp" => {
                let parts: Vec<&str> = command.split(':').collect();
                let host = parts
                    .first()
                    .map(|s| (*s).to_string())
                    .unwrap_or_else(|| "localhost".to_string());
                let port: u16 = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(8080);
                McpConnection {
                    connection_type: ConnectionType::Tcp,
                    url: host,
                    port: Some(port),
                    timeout_seconds: 30,
                    max_retries: 3,
                    retry_backoff_ms: 1000,
                    keep_alive: true,
                    tls_config: None,
                }
            }
            _ => {
                errors.push(McpLoadError {
                    line: line_num,
                    message: format!(
                        "Unknown connection type '{}' for server '{}'. Use: stdio, http, websocket, tcp",
                        conn_type, name
                    ),
                    recoverable: true,
                });
                return None;
            }
        };

        let auth = match (auth_type.as_deref(), auth_env.as_ref()) {
            (Some("api_key"), Some(env)) => {
                use super::mcp_client::{McpAuthType, McpCredentials};
                McpAuth {
                    auth_type: McpAuthType::ApiKey,
                    credentials: McpCredentials::ApiKey {
                        header_name: "X-API-Key".to_string(),
                        key_ref: env.clone(),
                    },
                }
            }
            (Some("bearer"), Some(env)) => {
                use super::mcp_client::{McpAuthType, McpCredentials};
                McpAuth {
                    auth_type: McpAuthType::Bearer,
                    credentials: McpCredentials::Bearer {
                        token_ref: env.clone(),
                    },
                }
            }
            _ => McpAuth::default(),
        };

        let server_type = Self::infer_server_type(&name, &conn_type, &command);

        let status = if enabled {
            McpServerStatus::Active
        } else {
            McpServerStatus::Inactive
        };

        debug!(
            "Loaded MCP server '{}' (type={}, enabled={})",
            name, server_type, enabled
        );

        Some(McpServer {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            server_type,
            connection,
            auth,
            tools: Vec::new(),
            capabilities: McpCapabilities {
                tools: true,
                resources: false,
                prompts: false,
                logging: false,
                streaming: false,
                cancellation: false,
                custom: HashMap::new(),
            },
            status,
            bot_id: self.bot_id.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_health_check: None,
            health_status: HealthStatus::default(),
        })
    }
}
