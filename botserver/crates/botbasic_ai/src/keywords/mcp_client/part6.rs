use super::*;

/// Launchers a registered MCP stdio server may be started with.
///
/// The stdio command comes from a bot-registered, database-backed
/// configuration, so it is untrusted input. `SafeCommand` matches on the
/// command basename against a server-wide allowlist that necessarily contains
/// shells (`sh`, `bash`, `cmd`, `powershell`) and `sudo` — which would turn
/// "register an MCP server" into arbitrary host command execution. Restricting
/// stdio transports to package and runtime launchers keeps the blast radius at
/// "run this published MCP package".
const MCP_STDIO_LAUNCHERS: &[&str] = &[
    "npx", "uvx", "uv", "node", "deno", "bun", "python", "python3", "docker",
];

/// Validates the stdio launcher of an MCP server, accepting a bare name or an
/// absolute path whose basename is a known launcher.
pub(crate) fn validate_stdio_command(
    command: &str,
) -> Result<&str, Box<dyn std::error::Error + Send + Sync>> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(anyhow::anyhow!("MCP stdio command must not be empty").into());
    }
    let basename = std::path::Path::new(trimmed)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(trimmed);
    if MCP_STDIO_LAUNCHERS.contains(&basename) {
        return Ok(trimmed);
    }
    Err(anyhow::anyhow!(
        "MCP stdio command '{basename}' is not an accepted launcher ({}); \
         register the server with a package runner such as npx or uvx",
        MCP_STDIO_LAUNCHERS.join(", ")
    )
    .into())
}

impl McpClient {
    pub(crate) async fn invoke_http(
        &self,
        server: &McpServer,
        request: &McpRequest,
    ) -> Result<McpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/tools/call", server.connection.url);

        let body = serde_json::json!({
            "name": request.tool,
            "arguments": request.arguments
        });

        let timeout = request
            .timeout_seconds
            .unwrap_or(server.connection.timeout_seconds);

        let mut http_request = self
            .http_client
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(timeout as u64));

        http_request = Self::add_auth_headers(http_request, &server.auth);

        let response = http_request.send().await?;
        let status = response.status();

        if status.is_success() {
            let result: serde_json::Value = response.json().await?;
            Ok(McpResponse {
                id: request.id.clone(),
                success: true,
                result: Some(result),
                error: None,
                metadata: McpResponseMetadata {
                    duration_ms: 0,
                    server_version: None,
                    rate_limit_remaining: None,
                    rate_limit_reset: None,
                },
            })
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Ok(McpResponse {
                id: request.id.clone(),
                success: false,
                result: None,
                error: Some(McpError {
                    code: format!("HTTP_{}", status.as_u16()),
                    message: error_text,
                    details: None,
                    retryable: status.as_u16() >= 500,
                }),
                metadata: McpResponseMetadata {
                    duration_ms: 0,
                    server_version: None,
                    rate_limit_remaining: None,
                    rate_limit_reset: None,
                },
            })
        }
    }

    pub(crate) async fn invoke_stdio(
        &self,
        server: &McpServer,
        request: &McpRequest,
    ) -> Result<McpResponse, Box<dyn std::error::Error + Send + Sync>> {
        let _input = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": request.tool,
                "arguments": request.arguments
            },
            "id": request.id
        });

        let command = validate_stdio_command(&server.connection.url)?;
        let cmd = SafeCommand::new(command)
            .map_err(|e| anyhow::anyhow!("Failed to build MCP command: {}", e))?;

        let output = cmd.execute_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute MCP command: {}", e))?;

        if output.status.success() {
            let result: serde_json::Value = serde_json::from_slice(&output.stdout)?;
            Ok(McpResponse {
                id: request.id.clone(),
                success: true,
                result: result.get("result").cloned(),
                error: None,
                metadata: McpResponseMetadata {
                    duration_ms: 0,
                    server_version: None,
                    rate_limit_remaining: None,
                    rate_limit_reset: None,
                },
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(McpResponse {
                id: request.id.clone(),
                success: false,
                result: None,
                error: Some(McpError {
                    code: "STDIO_ERROR".to_string(),
                    message: stderr.to_string(),
                    details: None,
                    retryable: false,
                }),
                metadata: McpResponseMetadata {
                    duration_ms: 0,
                    server_version: None,
                    rate_limit_remaining: None,
                    rate_limit_reset: None,
                },
            })
        }
    }

    pub(super) fn add_auth_headers(
        mut request: reqwest::RequestBuilder,
        auth: &McpAuth,
    ) -> reqwest::RequestBuilder {
        match &auth.credentials {
            McpCredentials::ApiKey {
                header_name,
                key_ref,
            } => {
                request = request.header(header_name.as_str(), key_ref.as_str());
            }
            McpCredentials::Bearer { token_ref } => {
                request = request.bearer_auth(token_ref);
            }
            McpCredentials::Basic {
                username_ref,
                password_ref,
            } => {
                request = request.basic_auth(username_ref, Some(password_ref));
            }
            _ => {}
        }
        request
    }

    pub async fn health_check(
        &mut self,
        server_name: &str,
    ) -> Result<HealthStatus, Box<dyn std::error::Error + Send + Sync>> {
        let server = self
            .servers
            .get_mut(server_name)
            .ok_or_else(|| format!("MCP server '{}' not found", server_name))?;

        let start_time = std::time::Instant::now();

        let health_url = format!("{}/health", server.connection.url);
        let result = self
            .http_client
            .get(&health_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        let latency_ms = start_time.elapsed().as_millis() as i64;

        match result {
            Ok(response) => {
                if response.status().is_success() {
                    server.status = McpServerStatus::Active;
                    Ok(HealthStatus {
                        healthy: true,
                        last_check: Some(Utc::now()),
                        response_time_ms: Some(latency_ms),
                        error_message: None,
                        consecutive_failures: 0,
                    })
                } else {
                    server.status = McpServerStatus::Error(format!("HTTP {}", response.status()));
                    Ok(HealthStatus {
                        healthy: false,
                        last_check: Some(Utc::now()),
                        response_time_ms: Some(latency_ms),
                        error_message: Some(format!(
                            "Server returned status {}",
                            response.status()
                        )),
                        consecutive_failures: 1,
                    })
                }
            }
            Err(e) => {
                server.status = McpServerStatus::Unknown;
                Ok(HealthStatus {
                    healthy: false,
                    last_check: Some(Utc::now()),
                    response_time_ms: Some(latency_ms),
                    error_message: Some(format!("Health check failed: {}", e)),
                    consecutive_failures: 1,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_stdio_command;

    #[test]
    fn accepts_package_and_runtime_launchers() {
        for command in [
            "npx",
            "uvx",
            "uv",
            "node",
            "deno",
            "bun",
            "python3",
            "/usr/bin/npx",
            " /opt/tools/uvx ",
        ] {
            assert!(
                validate_stdio_command(command).is_ok(),
                "{command} should be an accepted MCP launcher"
            );
        }
    }

    #[test]
    fn rejects_shells_and_prives_of_the_server_allowlist() {
        for command in [
            "sh", "bash", "cmd", "powershell", "sudo", "curl", "rm", "nc", "/bin/sh",
            "/usr/bin/sudo", "",
        ] {
            assert!(
                validate_stdio_command(command).is_err(),
                "{command} must not be accepted as an MCP launcher"
            );
        }
    }
}
