use super::*;

impl McpCsvLoader {
    pub(super) fn parse_csv_columns(line: &str) -> Vec<String> {
        let mut columns = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' if !in_quotes => {
                    in_quotes = true;
                }
                '"' if in_quotes => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        current.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                ',' if !in_quotes => {
                    columns.push(current.clone());
                    current.clear();
                }
                _ => {
                    current.push(c);
                }
            }
        }
        columns.push(current);

        columns
    }

    pub(super) fn parse_args(args: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';

        for c in args.chars() {
            match c {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = c;
                }
                c if in_quotes && c == quote_char => {
                    in_quotes = false;
                }
                ' ' if !in_quotes => {
                    if !current.is_empty() {
                        result.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            }
        }
        if !current.is_empty() {
            result.push(current);
        }

        result
    }

    pub(super) fn infer_server_type(name: &str, conn_type: &str, command: &str) -> McpServerType {
        let name_lower = name.to_lowercase();
        let cmd_lower = command.to_lowercase();

        if name_lower.contains("filesystem") || cmd_lower.contains("filesystem") {
            McpServerType::Filesystem
        } else if name_lower.contains("github") || cmd_lower.contains("github") {
            McpServerType::Web
        } else if name_lower.contains("postgres")
            || name_lower.contains("mysql")
            || name_lower.contains("sqlite")
            || name_lower.contains("database")
            || cmd_lower.contains("postgres")
            || cmd_lower.contains("mysql")
            || cmd_lower.contains("sqlite")
        {
            McpServerType::Database
        } else if name_lower.contains("slack") || cmd_lower.contains("slack") {
            McpServerType::Slack
        } else if name_lower.contains("teams") || cmd_lower.contains("teams") {
            McpServerType::Teams
        } else if name_lower.contains("email")
            || name_lower.contains("smtp")
            || name_lower.contains("imap")
        {
            McpServerType::Email
        } else if name_lower.contains("analytics") {
            McpServerType::Analytics
        } else if name_lower.contains("search") {
            McpServerType::Search
        } else if name_lower.contains("storage")
            || name_lower.contains("s3")
            || name_lower.contains("minio")
        {
            McpServerType::Storage
        } else if conn_type == "http" || conn_type == "websocket" {
            McpServerType::Web
        } else {
            McpServerType::Custom("custom".to_string())
        }
    }

    pub fn load_server(&self, name: &str) -> Option<McpServer> {
        let result = self.load();
        result.servers.into_iter().find(|s| s.name == name)
    }

    pub fn create_example_csv(&self) -> std::io::Result<PathBuf> {
        let csv_path = self.get_csv_path();

        if let Some(parent) = csv_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let example_content = r#"name,type,command,args,description,enabled
# MCP Server Configuration
# Columns: name,type,command,args,description,enabled,auth_type,auth_env,risk_level,requires_approval
#
# type: stdio (local process), http (REST API), websocket, tcp
# auth_type: none, api_key, bearer
# risk_level: safe, low, medium, high, critical
#
# Example servers:
filesystem,stdio,npx,"-y @modelcontextprotocol/server-filesystem /data",Access local files and directories,true
# github,stdio,npx,"-y @modelcontextprotocol/server-github",GitHub API integration,true,bearer,GITHUB_TOKEN
# postgres,stdio,npx,"-y @modelcontextprotocol/server-postgres",PostgreSQL database queries,false
# slack,stdio,npx,"-y @modelcontextprotocol/server-slack",Slack messaging,false,bearer,SLACK_BOT_TOKEN
# myapi,http,https://api.example.com/mcp,,Custom API server,true,api_key,MY_API_KEY
"#;

        std::fs::write(&csv_path, example_content)?;
        info!("Created example mcp.csv at {}", csv_path.display());

        Ok(csv_path)
    }

    pub fn add_server(&self, row: &McpCsvRow) -> std::io::Result<()> {
        let csv_path = self.get_csv_path();

        if !csv_path.exists() {
            if let Some(parent) = csv_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&csv_path, "name,type,command,args,description,enabled,auth_type,auth_env,risk_level,requires_approval\n")?;
        }

        let line = format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            Self::escape_csv(&row.name),
            Self::escape_csv(&row.connection_type),
            Self::escape_csv(&row.command),
            Self::escape_csv(&row.args),
            Self::escape_csv(&row.description),
            row.enabled,
            row.auth_type.as_deref().unwrap_or(""),
            row.auth_env.as_deref().unwrap_or(""),
            row.risk_level.as_deref().unwrap_or("medium"),
            row.requires_approval
        );

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&csv_path)?;
        file.write_all(line.as_bytes())?;

        info!("Added MCP server '{}' to {}", row.name, csv_path.display());
        Ok(())
    }

    pub fn remove_server(&self, name: &str) -> std::io::Result<bool> {
        let csv_path = self.get_csv_path();

        if !csv_path.exists() {
            return Ok(false);
        }

        let content = std::fs::read_to_string(&csv_path)?;
        let mut new_lines: Vec<&str> = Vec::new();
        let mut found = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                new_lines.push(line);
                continue;
            }

            let columns = Self::parse_csv_columns(line);
            if columns.first().map(|s| s.trim()) == Some(name) {
                found = true;
                continue;
            }

            new_lines.push(line);
        }

        if found {
            std::fs::write(&csv_path, new_lines.join("\n") + "\n")?;
            info!("Removed MCP server '{}' from {}", name, csv_path.display());
        }

        Ok(found)
    }

    pub(super) fn escape_csv(value: &str) -> String {
        if value.contains(',') || value.contains('"') || value.contains('\n') {
            format!("\"{}\"", value.replace('"', "\"\""))
        } else {
            value.to_string()
        }
    }
}
