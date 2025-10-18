use serde::{Deserialize, Serialize};
use std::error::Error;

/// Generate API endpoint handler code for a tool
pub fn generate_endpoint_handler(
    tool_name: &str,
    parameters: &[crate::basic::compiler::ParamDeclaration],
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut handler_code = String::new();

    // Generate function signature
    handler_code.push_str(&format!(
        "// Auto-generated endpoint handler for tool: {}\n",
        tool_name
    ));
    handler_code.push_str(&format!(
        "pub async fn {}_handler(\n",
        tool_name.to_lowercase()
    ));
    handler_code.push_str("    state: web::Data<Arc<AppState>>,\n");
    handler_code.push_str(&format!(
        "    req: web::Json<{}Request>,\n",
        to_pascal_case(tool_name)
    ));
    handler_code.push_str(&format!(") -> Result<HttpResponse, actix_web::Error> {{\n"));

    // Generate handler body
    handler_code.push_str("    // Validate input parameters\n");
    for param in parameters {
        if param.required {
            handler_code.push_str(&format!(
                "    if req.{}.is_empty() {{\n",
                param.name.to_lowercase()
            ));
            handler_code.push_str(&format!(
                "        return Ok(HttpResponse::BadRequest().json(json!({{\"error\": \"Missing required parameter: {}\"}})));\n",
                param.name
            ));
            handler_code.push_str("    }\n");
        }
    }

    handler_code.push_str("\n    // Execute BASIC script\n");
    handler_code.push_str(&format!(
        "    let script_path = \"./work/default.gbai/default.gbdialog/{}.ast\";\n",
        tool_name
    ));
    handler_code.push_str("    // TODO: Load and execute AST\n");
    handler_code.push_str("\n    Ok(HttpResponse::Ok().json(json!({\"status\": \"success\"})))\n");
    handler_code.push_str("}\n\n");

    // Generate request structure
    handler_code.push_str(&generate_request_struct(tool_name, parameters)?);

    Ok(handler_code)
}

/// Generate request struct for tool
fn generate_request_struct(
    tool_name: &str,
    parameters: &[crate::basic::compiler::ParamDeclaration],
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut struct_code = String::new();

    struct_code.push_str(&format!(
        "#[derive(Debug, Clone, Serialize, Deserialize)]\n"
    ));
    struct_code.push_str(&format!(
        "pub struct {}Request {{\n",
        to_pascal_case(tool_name)
    ));

    for param in parameters {
        let rust_type = param_type_to_rust_type(&param.param_type);

        if param.required {
            struct_code.push_str(&format!(
                "    pub {}: {},\n",
                param.name.to_lowercase(),
                rust_type
            ));
        } else {
            struct_code.push_str(&format!(
                "    #[serde(skip_serializing_if = \"Option::is_none\")]\n"
            ));
            struct_code.push_str(&format!(
                "    pub {}: Option<{}>,\n",
                param.name.to_lowercase(),
                rust_type
            ));
        }
    }

    struct_code.push_str("}\n");

    Ok(struct_code)
}

/// Convert parameter type to Rust type
fn param_type_to_rust_type(param_type: &str) -> String {
    match param_type {
        "string" => "String".to_string(),
        "integer" => "i64".to_string(),
        "number" => "f64".to_string(),
        "boolean" => "bool".to_string(),
        "array" => "Vec<serde_json::Value>".to_string(),
        "object" => "serde_json::Value".to_string(),
        _ => "String".to_string(),
    }
}

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Generate route registration code
pub fn generate_route_registration(tool_name: &str) -> String {
    format!(
        "    .service(web::resource(\"/default/{}\").route(web::post().to({}_handler)))\n",
        tool_name,
        tool_name.to_lowercase()
    )
}

/// Tool metadata for MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPServerInfo {
    pub name: String,
    pub version: String,
    pub tools: Vec<MCPToolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPToolInfo {
    pub name: String,
    pub description: String,
    pub endpoint: String,
}

/// Generate MCP server manifest
pub fn generate_mcp_server_manifest(
    tools: Vec<MCPToolInfo>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let manifest = MCPServerInfo {
        name: "GeneralBots BASIC MCP Server".to_string(),
        version: "1.0.0".to_string(),
        tools,
    };

    let json = serde_json::to_string_pretty(&manifest)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basic::compiler::ParamDeclaration;

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("enrollment"), "Enrollment");
        assert_eq!(to_pascal_case("pricing_tool"), "PricingTool");
        assert_eq!(to_pascal_case("get_user_data"), "GetUserData");
    }

    #[test]
    fn test_param_type_to_rust_type() {
        assert_eq!(param_type_to_rust_type("string"), "String");
        assert_eq!(param_type_to_rust_type("integer"), "i64");
        assert_eq!(param_type_to_rust_type("number"), "f64");
        assert_eq!(param_type_to_rust_type("boolean"), "bool");
        assert_eq!(param_type_to_rust_type("array"), "Vec<serde_json::Value>");
    }

    #[test]
    fn test_generate_request_struct() {
        let params = vec![
            ParamDeclaration {
                name: "name".to_string(),
                param_type: "string".to_string(),
                example: Some("John Doe".to_string()),
                description: "User name".to_string(),
                required: true,
            },
            ParamDeclaration {
                name: "age".to_string(),
                param_type: "integer".to_string(),
                example: Some("25".to_string()),
                description: "User age".to_string(),
                required: false,
            },
        ];

        let result = generate_request_struct("test_tool", &params).unwrap();

        assert!(result.contains("pub struct TestToolRequest"));
        assert!(result.contains("pub name: String"));
        assert!(result.contains("pub age: Option<i64>"));
    }

    #[test]
    fn test_generate_route_registration() {
        let route = generate_route_registration("enrollment");
        assert!(route.contains("/default/enrollment"));
        assert!(route.contains("enrollment_handler"));
    }
}
