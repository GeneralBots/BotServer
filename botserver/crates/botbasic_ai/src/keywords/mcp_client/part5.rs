use super::*;

impl McpClient {
    pub fn new(state: Arc<dyn BasicRuntime>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            state,
            config: McpClientConfig::default(),
            servers: HashMap::new(),
            http_client,
        }
    }

    pub fn with_config(state: Arc<dyn BasicRuntime>, config: McpClientConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.default_timeout_seconds as u64))
            .build()
            .unwrap_or_default();

        Self {
            state,
            config,
            servers: HashMap::new(),
            http_client,
        }
    }

    pub fn load_servers(
        &mut self,
        bot_id: &Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("DB error: {}", e))?;
        let bot_id_str = bot_id.to_string();

        let query = diesel::sql_query(
            "SELECT id, name, description, server_type, config, status, created_at, updated_at
             FROM mcp_servers WHERE bot_id = $1 AND status != 'deleted'",
        )
        .bind::<diesel::sql_types::Text, _>(&bot_id_str);

        #[derive(QueryableByName)]
        struct ServerRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            id: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            description: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Text)]
            server_type: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            config: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            status: String,
        }

        let rows: Vec<ServerRow> = query.load(&mut *conn).unwrap_or_default();

        for row in rows {
            let server = McpServer {
                id: row.id.clone(),
                name: row.name.clone(),
                description: row.description.unwrap_or_default(),
                server_type: McpServerType::from(row.server_type.as_str()),
                connection: serde_json::from_str(&row.config).unwrap_or_default(),
                auth: McpAuth::default(),
                tools: Vec::new(),
                capabilities: McpCapabilities::default(),
                status: match row.status.as_str() {
                    "active" => McpServerStatus::Active,
                    "maintenance" => McpServerStatus::Maintenance,
                    "error" => McpServerStatus::Error("Unknown error".to_string()),
                    _ => McpServerStatus::Inactive,
                },
                bot_id: bot_id_str.clone(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_health_check: None,
                health_status: HealthStatus::default(),
            };

            self.servers.insert(row.name, server);
        }

        info!(
            "Loaded {} MCP servers for bot {}",
            self.servers.len(),
            bot_id
        );
        Ok(())
    }

    pub fn register_server(
        &mut self,
        server: McpServer,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("DB error: {}", e))?;

        let config_json = serde_json::to_string(&server.connection)?;
        let now = Utc::now().to_rfc3339();

        let server_type_str = server.server_type.to_string();
        let query = diesel::sql_query(
            "INSERT INTO mcp_servers (id, bot_id, name, description, server_type, config, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (bot_id, name) DO UPDATE SET
                description = EXCLUDED.description,
                server_type = EXCLUDED.server_type,
                config = EXCLUDED.config,
                status = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at"
        )
            .bind::<diesel::sql_types::Text, _>(&server.id)
            .bind::<diesel::sql_types::Text, _>(&server.bot_id)
            .bind::<diesel::sql_types::Text, _>(&server.name)
            .bind::<diesel::sql_types::Text, _>(&server.description)
            .bind::<diesel::sql_types::Text, _>(&server_type_str)
            .bind::<diesel::sql_types::Text, _>(&config_json)
            .bind::<diesel::sql_types::Text, _>("active")
            .bind::<diesel::sql_types::Text, _>(&now)
            .bind::<diesel::sql_types::Text, _>(&now);

        query
            .execute(&mut *conn)
            .map_err(|e| format!("Failed to register MCP server: {}", e))?;

        self.servers.insert(server.name.clone(), server);
        Ok(())
    }

    pub fn get_server(&self, name: &str) -> Option<&McpServer> {
        self.servers.get(name)
    }

    pub fn list_servers(&self) -> Vec<&McpServer> {
        self.servers.values().collect()
    }

    pub async fn list_tools(
        &self,
        server_name: &str,
    ) -> Result<Vec<McpTool>, Box<dyn std::error::Error + Send + Sync>> {
        let server = self
            .servers
            .get(server_name)
            .ok_or_else(|| format!("MCP server '{}' not found", server_name))?;

        if server.connection.connection_type == ConnectionType::Http {
            let url = format!("{}/tools/list", server.connection.url);
            let response = self
                .http_client
                .get(&url)
                .timeout(Duration::from_secs(
                    server.connection.timeout_seconds as u64,
                ))
                .send()
                .await?;

            if response.status().is_success() {
                let tools: Vec<McpTool> = response.json().await?;
                return Ok(tools);
            }
        }

        Ok(server.tools.clone())
    }

    pub async fn invoke_tool(
        &self,
        request: McpRequest,
    ) -> Result<McpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = std::time::Instant::now();

        let server = self
            .servers
            .get(&request.server)
            .ok_or_else(|| format!("MCP server '{}' not found", request.server))?;

        if server.status != McpServerStatus::Active {
            return Ok(McpResponse {
                id: request.id,
                success: false,
                result: None,
                error: Some(McpError {
                    code: "SERVER_UNAVAILABLE".to_string(),
                    message: format!(
                        "MCP server '{}' is not active (status: {:?})",
                        request.server, server.status
                    ),
                    details: None,
                    retryable: true,
                }),
                metadata: McpResponseMetadata {
                    duration_ms: start_time.elapsed().as_millis() as i64,
                    server_version: None,
                    rate_limit_remaining: None,
                    rate_limit_reset: None,
                },
            });
        }

        if self.config.audit_enabled {
            info!(
                "MCP request: server={} tool={}",
                request.server, request.tool
            );
        }

        let result = match server.connection.connection_type {
            ConnectionType::Http => self.invoke_http(server, &request).await,
            ConnectionType::Stdio => self.invoke_stdio(server, &request).await,
            _ => Err(format!(
                "Connection type {:?} not yet supported",
                server.connection.connection_type
            )
            .into()),
        };

        let duration_ms = start_time.elapsed().as_millis() as i64;

        match result {
            Ok(mut response) => {
                response.metadata.duration_ms = duration_ms;

                if self.config.audit_enabled {
                    info!(
                        "MCP response: id={} success={}",
                        response.id, response.success
                    );
                }

                Ok(response)
            }
            Err(e) => {
                let response = McpResponse {
                    id: request.id.clone(),
                    success: false,
                    result: None,
                    error: Some(McpError {
                        code: "INVOCATION_ERROR".to_string(),
                        message: e.to_string(),
                        details: None,
                        retryable: true,
                    }),
                    metadata: McpResponseMetadata {
                        duration_ms,
                        server_version: None,
                        rate_limit_remaining: None,
                        rate_limit_reset: None,
                    },
                };

                if self.config.audit_enabled {
                    info!(
                        "MCP error response: id={} error={:?}",
                        response.id, response.error
                    );
                }

                Ok(response)
            }
        }
    }
}
