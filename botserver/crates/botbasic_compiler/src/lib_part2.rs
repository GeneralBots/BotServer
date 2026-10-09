use super::*;

impl BasicCompiler {
    pub fn new(state: Arc<AppState>, bot_id: Uuid) -> Self {
        Self {
            state,
            bot_id,
            previous_schedules: std::collections::HashSet::new(),
            callbacks: CompilerCallbacks::new(),
        }
    }

    pub fn with_callbacks(state: Arc<AppState>, bot_id: Uuid, callbacks: CompilerCallbacks) -> Self {
        Self {
            state,
            bot_id,
            previous_schedules: std::collections::HashSet::new(),
            callbacks,
        }
    }

    pub fn compile_file(
        &mut self,
        source_path: &str,
        output_dir: &str,
    ) -> Result<CompilationResult, Box<dyn std::error::Error + Send + Sync>> {
        let source_content = std::fs::read_to_string(source_path)
            .map_err(|e| format!("Failed to read source file: {e}"))?;

        let should_process_tables = if source_path.contains("tables.bas") {
            let work_path = botcore::shared::utils::get_work_path();
            let bot_name = Self::get_bot_name_from_state(&self.state, self.bot_id)?;
            let tables_bas_path = format!(
                "{work_path}/{org_id}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/tables.bas",
                org_id = current_org_id()
            );
            let tables_ast_path = tables_bas_path.replace(".bas", ".ast");

            match (
                std::fs::metadata(&tables_bas_path).ok(),
                std::fs::metadata(&tables_ast_path).ok(),
            ) {
                (Some(bas_meta), Some(ast_meta)) => {
                    bas_meta.modified().ok() > ast_meta.modified().ok()
                }
                _ => true,
            }
        } else {
            false
        };

        if should_process_tables {
            if let Err(e) = self.process_tables_bas() {
                log::warn!("Failed to process tables.bas: {}", e);
            }
            let tool_def = Self::parse_tool_definition(&source_content, source_path)?;
            let file_name = std::path::Path::new(source_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or("Invalid file name")?;
            let source_with_suggestions =
                self.generate_enum_suggestions(&source_content, &tool_def)?;
            let ast_path = format!("{output_dir}/{file_name}.ast");
            let ast_content =
                self.preprocess_basic(&source_with_suggestions, source_path, self.bot_id)?;
            std::fs::write(&ast_path, &ast_content).map_err(|e| format!("Failed to write AST: {e}"))?;
            return Ok(CompilationResult {
                mcp_tool: None,
                openai_tool: None,
            });
        }

        if let Some(ref process_tables) = self.callbacks.process_table_definitions {
            if let Some(ref create_runtime) = self.callbacks.create_runtime {
                let runtime = (create_runtime)(Arc::clone(&self.state));
                if let Err(e) = (process_tables)(runtime, self.bot_id, &source_content) {
                    log::warn!("Failed to process TABLE definitions: {}", e);
                }
            }
        }

        let tool_def = Self::parse_tool_definition(&source_content, source_path)?;
        let file_name = std::path::Path::new(source_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid file name")?;

        let source_with_suggestions = self.generate_enum_suggestions(&source_content, &tool_def)?;

        let ast_path = format!("{output_dir}/{file_name}.ast");
        let ast_content =
            self.preprocess_basic(&source_with_suggestions, source_path, self.bot_id)?;
        std::fs::write(&ast_path, &ast_content).map_err(|e| format!("Failed to write AST file: {e}"))?;
        let (mcp_json, tool_json) = if tool_def.parameters.is_empty() {
            let mcp = Self::generate_mcp_tool(&tool_def)?;
            let openai = Self::generate_openai_tool(&tool_def)?;
            let mcp_path = format!("{output_dir}/{file_name}.mcp.json");
            let tool_path = format!("{output_dir}/{file_name}.tool.json");
            let mcp_json_str = serde_json::to_string_pretty(&mcp)?;
            std::fs::write(&mcp_path, mcp_json_str)
                .map_err(|e| format!("Failed to write MCP JSON: {e}"))?;
            let tool_json_str = serde_json::to_string_pretty(&openai)?;
            std::fs::write(&tool_path, tool_json_str)
                .map_err(|e| format!("Failed to write tool JSON: {e}"))?;
            (Some(mcp), Some(openai))
        } else {
            let mcp = Self::generate_mcp_tool(&tool_def)?;
            let openai = Self::generate_openai_tool(&tool_def)?;
            let mcp_path = format!("{output_dir}/{file_name}.mcp.json");
            let tool_path = format!("{output_dir}/{file_name}.tool.json");
            let mcp_json_str = serde_json::to_string_pretty(&mcp)?;
            std::fs::write(&mcp_path, mcp_json_str)
                .map_err(|e| format!("Failed to write MCP JSON: {e}"))?;
            let tool_json_str = serde_json::to_string_pretty(&openai)?;
            std::fs::write(&tool_path, tool_json_str)
                .map_err(|e| format!("Failed to write tool JSON: {e}"))?;
            (Some(mcp), Some(openai))
        };
        Ok(CompilationResult {
            mcp_tool: mcp_json,
            openai_tool: tool_json,
        })
    }
}
