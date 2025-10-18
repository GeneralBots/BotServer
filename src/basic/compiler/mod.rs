use crate::shared::state::AppState;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub mod tool_generator;

/// Represents a PARAM declaration in BASIC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDeclaration {
    pub name: String,
    pub param_type: String,
    pub example: Option<String>,
    pub description: String,
    pub required: bool,
}

/// Represents a BASIC tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParamDeclaration>,
    pub source_file: String,
}

/// MCP tool format (Model Context Protocol)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPTool {
    pub name: String,
    pub description: String,
    pub input_schema: MCPInputSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, MCPProperty>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPProperty {
    #[serde(rename = "type")]
    pub prop_type: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

/// OpenAI tool format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAIFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    pub name: String,
    pub description: String,
    pub parameters: OpenAIParameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIParameters {
    #[serde(rename = "type")]
    pub param_type: String,
    pub properties: HashMap<String, OpenAIProperty>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIProperty {
    #[serde(rename = "type")]
    pub prop_type: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

/// BASIC Compiler
pub struct BasicCompiler {
    state: Arc<AppState>,
}

impl BasicCompiler {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Compile a BASIC file to AST and generate tool definitions
    pub fn compile_file(
        &self,
        source_path: &str,
        output_dir: &str,
    ) -> Result<CompilationResult, Box<dyn Error + Send + Sync>> {
        info!("Compiling BASIC file: {}", source_path);

        // Read source file
        let source_content = fs::read_to_string(source_path)
            .map_err(|e| format!("Failed to read source file: {}", e))?;

        // Parse tool definition from source
        let tool_def = self.parse_tool_definition(&source_content, source_path)?;

        // Extract base name without extension
        let file_name = Path::new(source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid file name")?;

        // Generate AST path
        let ast_path = format!("{}/{}.ast", output_dir, file_name);

        // Generate AST (using Rhai compilation would happen here)
        // For now, we'll store the preprocessed script
        let ast_content = self.preprocess_basic(&source_content)?;
        fs::write(&ast_path, &ast_content)
            .map_err(|e| format!("Failed to write AST file: {}", e))?;

        info!("AST generated: {}", ast_path);

        // Generate tool definitions if PARAM and DESCRIPTION found
        let (mcp_json, tool_json) = if !tool_def.parameters.is_empty() {
            let mcp = self.generate_mcp_tool(&tool_def)?;
            let openai = self.generate_openai_tool(&tool_def)?;

            let mcp_path = format!("{}/{}.mcp.json", output_dir, file_name);
            let tool_path = format!("{}/{}.tool.json", output_dir, file_name);

            // Write MCP JSON
            let mcp_json_str = serde_json::to_string_pretty(&mcp)?;
            fs::write(&mcp_path, mcp_json_str)
                .map_err(|e| format!("Failed to write MCP JSON: {}", e))?;

            // Write OpenAI tool JSON
            let tool_json_str = serde_json::to_string_pretty(&openai)?;
            fs::write(&tool_path, tool_json_str)
                .map_err(|e| format!("Failed to write tool JSON: {}", e))?;

            info!("Tool definitions generated: {} and {}", mcp_path, tool_path);

            (Some(mcp), Some(openai))
        } else {
            debug!("No tool parameters found in {}", source_path);
            (None, None)
        };

        Ok(CompilationResult {
            ast_path,
            mcp_tool: mcp_json,
            openai_tool: tool_json,
            tool_definition: Some(tool_def),
        })
    }

    /// Parse tool definition from BASIC source
    fn parse_tool_definition(
        &self,
        source: &str,
        source_path: &str,
    ) -> Result<ToolDefinition, Box<dyn Error + Send + Sync>> {
        let mut params = Vec::new();
        let mut description = String::new();

        let lines: Vec<&str> = source.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();

            // Parse PARAM declarations
            if line.starts_with("PARAM ") {
                if let Some(param) = self.parse_param_line(line)? {
                    params.push(param);
                }
            }

            // Parse DESCRIPTION
            if line.starts_with("DESCRIPTION ") {
                let desc_start = line.find('"').unwrap_or(0);
                let desc_end = line.rfind('"').unwrap_or(line.len());
                if desc_start < desc_end {
                    description = line[desc_start + 1..desc_end].to_string();
                }
            }

            i += 1;
        }

        let tool_name = Path::new(source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(ToolDefinition {
            name: tool_name,
            description,
            parameters: params,
            source_file: source_path.to_string(),
        })
    }

    /// Parse a PARAM line
    /// Format: PARAM name AS type LIKE "example" DESCRIPTION "description"
    fn parse_param_line(
        &self,
        line: &str,
    ) -> Result<Option<ParamDeclaration>, Box<dyn Error + Send + Sync>> {
        let line = line.trim();
        if !line.starts_with("PARAM ") {
            return Ok(None);
        }

        // Extract parts
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            warn!("Invalid PARAM line: {}", line);
            return Ok(None);
        }

        let name = parts[1].to_string();

        // Find AS keyword
        let as_index = parts.iter().position(|&p| p == "AS");
        let param_type = if let Some(idx) = as_index {
            if idx + 1 < parts.len() {
                parts[idx + 1].to_lowercase()
            } else {
                "string".to_string()
            }
        } else {
            "string".to_string()
        };

        // Extract LIKE value (example)
        let example = if let Some(like_pos) = line.find("LIKE") {
            let rest = &line[like_pos + 4..].trim();
            if let Some(start) = rest.find('"') {
                if let Some(end) = rest[start + 1..].find('"') {
                    Some(rest[start + 1..start + 1 + end].to_string())
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Extract DESCRIPTION
        let description = if let Some(desc_pos) = line.find("DESCRIPTION") {
            let rest = &line[desc_pos + 11..].trim();
            if let Some(start) = rest.find('"') {
                if let Some(end) = rest[start + 1..].rfind('"') {
                    rest[start + 1..start + 1 + end].to_string()
                } else {
                    "".to_string()
                }
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        Ok(Some(ParamDeclaration {
            name,
            param_type: self.normalize_type(&param_type),
            example,
            description,
            required: true, // Default to required
        }))
    }

    /// Normalize BASIC types to JSON schema types
    fn normalize_type(&self, basic_type: &str) -> String {
        match basic_type.to_lowercase().as_str() {
            "string" | "text" => "string".to_string(),
            "integer" | "int" | "number" => "integer".to_string(),
            "float" | "double" | "decimal" => "number".to_string(),
            "boolean" | "bool" => "boolean".to_string(),
            "date" | "datetime" => "string".to_string(), // Dates as strings
            "array" | "list" => "array".to_string(),
            "object" | "map" => "object".to_string(),
            _ => "string".to_string(), // Default to string
        }
    }

    /// Generate MCP tool format
    fn generate_mcp_tool(
        &self,
        tool_def: &ToolDefinition,
    ) -> Result<MCPTool, Box<dyn Error + Send + Sync>> {
        let mut properties = HashMap::new();
        let mut required = Vec::new();

        for param in &tool_def.parameters {
            properties.insert(
                param.name.clone(),
                MCPProperty {
                    prop_type: param.param_type.clone(),
                    description: param.description.clone(),
                    example: param.example.clone(),
                },
            );

            if param.required {
                required.push(param.name.clone());
            }
        }

        Ok(MCPTool {
            name: tool_def.name.clone(),
            description: tool_def.description.clone(),
            input_schema: MCPInputSchema {
                schema_type: "object".to_string(),
                properties,
                required,
            },
        })
    }

    /// Generate OpenAI tool format
    fn generate_openai_tool(
        &self,
        tool_def: &ToolDefinition,
    ) -> Result<OpenAITool, Box<dyn Error + Send + Sync>> {
        let mut properties = HashMap::new();
        let mut required = Vec::new();

        for param in &tool_def.parameters {
            properties.insert(
                param.name.clone(),
                OpenAIProperty {
                    prop_type: param.param_type.clone(),
                    description: param.description.clone(),
                    example: param.example.clone(),
                },
            );

            if param.required {
                required.push(param.name.clone());
            }
        }

        Ok(OpenAITool {
            tool_type: "function".to_string(),
            function: OpenAIFunction {
                name: tool_def.name.clone(),
                description: tool_def.description.clone(),
                parameters: OpenAIParameters {
                    param_type: "object".to_string(),
                    properties,
                    required,
                },
            },
        })
    }

    /// Preprocess BASIC script (basic transformations)
    fn preprocess_basic(&self, source: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut result = String::new();

        for line in source.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("REM") {
                continue;
            }

            // Skip PARAM and DESCRIPTION lines (metadata)
            if trimmed.starts_with("PARAM ") || trimmed.starts_with("DESCRIPTION ") {
                continue;
            }

            result.push_str(trimmed);
            result.push('\n');
        }

        Ok(result)
    }
}

/// Result of compilation
#[derive(Debug)]
pub struct CompilationResult {
    pub ast_path: String,
    pub mcp_tool: Option<MCPTool>,
    pub openai_tool: Option<OpenAITool>,
    pub tool_definition: Option<ToolDefinition>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_type() {
        let compiler = BasicCompiler::new(Arc::new(AppState::default()));

        assert_eq!(compiler.normalize_type("string"), "string");
        assert_eq!(compiler.normalize_type("integer"), "integer");
        assert_eq!(compiler.normalize_type("int"), "integer");
        assert_eq!(compiler.normalize_type("boolean"), "boolean");
        assert_eq!(compiler.normalize_type("date"), "string");
    }

    #[test]
    fn test_parse_param_line() {
        let compiler = BasicCompiler::new(Arc::new(AppState::default()));

        let line = r#"PARAM name AS string LIKE "John Doe" DESCRIPTION "User's full name""#;
        let result = compiler.parse_param_line(line).unwrap();

        assert!(result.is_some());
        let param = result.unwrap();
        assert_eq!(param.name, "name");
        assert_eq!(param.param_type, "string");
        assert_eq!(param.example, Some("John Doe".to_string()));
        assert_eq!(param.description, "User's full name");
    }
}
