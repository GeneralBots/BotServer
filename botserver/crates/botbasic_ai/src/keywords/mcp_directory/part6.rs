use super::*;

pub fn generate_example_configs() -> Vec<McpServerConfig> {
    vec![
        McpServerConfig {
            name: "filesystem".to_string(),
            description: "Access local files and directories".to_string(),
            server_type: "filesystem".to_string(),
            enabled: true,
            connection: McpConnectionConfig::Stdio {
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-filesystem".to_string(),
                    "/data".to_string(),
                ],
                cwd: None,
            },
            auth: None,
            tools: vec![
                McpToolConfig {
                    name: "read_file".to_string(),
                    description: "Read contents of a file".to_string(),
                    input_schema: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "File path to read"}
                        },
                        "required": ["path"]
                    })),
                    output_schema: None,
                    risk_level: Some("low".to_string()),
                    requires_approval: false,
                    rate_limit: None,
                },
                McpToolConfig {
                    name: "write_file".to_string(),
                    description: "Write contents to a file".to_string(),
                    input_schema: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "File path to write"},
                            "content": {"type": "string", "description": "Content to write"}
                        },
                        "required": ["path", "content"]
                    })),
                    output_schema: None,
                    risk_level: Some("medium".to_string()),
                    requires_approval: true,
                    rate_limit: None,
                },
                McpToolConfig {
                    name: "list_directory".to_string(),
                    description: "List files in a directory".to_string(),
                    input_schema: Some(serde_json::json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string", "description": "Directory path"}
                        },
                        "required": ["path"]
                    })),
                    output_schema: None,
                    risk_level: Some("safe".to_string()),
                    requires_approval: false,
                    rate_limit: None,
                },
            ],
            env: HashMap::new(),
            tags: vec!["storage".to_string(), "files".to_string()],
            risk_level: "low".to_string(),
            requires_approval: false,
        },
        McpServerConfig {
            name: "github".to_string(),
            description: "Interact with GitHub repositories".to_string(),
            server_type: "github".to_string(),
            enabled: true,
            connection: McpConnectionConfig::Stdio {
                command: "npx".to_string(),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-github".to_string(),
                ],
                cwd: None,
            },
            auth: Some(McpAuthConfig::Bearer {
                token_env: "GITHUB_TOKEN".to_string(),
            }),
            tools: vec![McpToolConfig {
                name: "search_repositories".to_string(),
                description: "Search GitHub repositories".to_string(),
                input_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"}
                    },
                    "required": ["query"]
                })),
                output_schema: None,
                risk_level: Some("safe".to_string()),
                requires_approval: false,
                rate_limit: Some(30),
            }],
            env: HashMap::new(),
            tags: vec!["vcs".to_string(), "github".to_string()],
            risk_level: "medium".to_string(),
            requires_approval: false,
        },
    ]
}
