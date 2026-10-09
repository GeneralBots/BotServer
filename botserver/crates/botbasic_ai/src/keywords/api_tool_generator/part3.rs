use super::*;

impl ApiToolGenerator {
    pub(super) fn generate_bas_file(api_name: &str, endpoint: &GeneratedEndpoint) -> Result<String, String> {
        let mut bas = String::new();

        let _ = writeln!(bas, "' Auto-generated tool for {} API", api_name);
        let _ = writeln!(bas, "' Endpoint: {} {}", endpoint.method, endpoint.path);
        let _ = writeln!(bas, "' Generated at: {}\n", chrono::Utc::now().to_rfc3339());

        for param in &endpoint.parameters {
            let example = param.example.as_deref().unwrap_or("");
            let required_marker = if param.required { "" } else { " ' optional" };

            let _ = writeln!(
                bas,
                "PARAM {} AS {} LIKE \"{}\" DESCRIPTION \"{}\"{}",
                Self::sanitize_param_name(&param.name),
                param.param_type,
                example,
                Self::escape_description(&param.description),
                required_marker
            );
        }

        let _ = writeln!(
            bas,
            "\nDESCRIPTION \"{}\"\n",
            Self::escape_description(&endpoint.description)
        );

        let mut url = format!("{}{}", endpoint.base_url, endpoint.path);
        let path_params: Vec<&EndpointParameter> = endpoint
            .parameters
            .iter()
            .filter(|p| p.location == "path")
            .collect();

        for param in &path_params {
            url = url.replace(
                &format!("{{{}}}", param.name),
                &format!("\" + {} + \"", Self::sanitize_param_name(&param.name)),
            );
        }

        let query_params: Vec<&EndpointParameter> = endpoint
            .parameters
            .iter()
            .filter(|p| p.location == "query")
            .collect();

        if !query_params.is_empty() {
            bas.push_str("' Build query string\n");
            bas.push_str("query_params = \"\"\n");
            for (i, param) in query_params.iter().enumerate() {
                let name = Self::sanitize_param_name(&param.name);
                let sep = if i == 0 { "?" } else { "&" };
                let _ = writeln!(
                    bas,
                    "IF NOT ISEMPTY({}) THEN query_params = query_params + \"{}{}=\" + {}",
                    name, sep, param.name, name
                );
            }
            bas.push('\n');
        }

        let body_params: Vec<&EndpointParameter> = endpoint
            .parameters
            .iter()
            .filter(|p| p.location == "body")
            .collect();

        if !body_params.is_empty() {
            bas.push_str("' Build request body\n");
            bas.push_str("body = {}\n");
            for param in &body_params {
                let name = Self::sanitize_param_name(&param.name);
                let _ = writeln!(
                    bas,
                    "IF NOT ISEMPTY({}) THEN body.{} = {}",
                    name, param.name, name
                );
            }
            bas.push('\n');
        }

        bas.push_str("' Make API request\n");
        let full_url = if query_params.is_empty() {
            format!("\"{}\"", url)
        } else {
            format!("\"{}\" + query_params", url)
        };

        if body_params.is_empty() {
            let _ = writeln!(bas, "result = {} HTTP {}", endpoint.method, full_url);
        } else {
            let _ = writeln!(
                bas,
                "result = {} HTTP {} WITH body",
                endpoint.method, full_url
            );
        }

        bas.push_str("\n' Return result\n");
        bas.push_str("RETURN result\n");

        Ok(bas)
    }

    pub(super) fn get_api_configs(&self) -> Result<Vec<(String, String)>, String> {
        let vault_configs = read_bot_config_from_vault(&self.bot_id)
            .ok_or_else(|| "Failed to read bot config from Vault".to_string())?;

        let result: Vec<(String, String)> = vault_configs
            .into_iter()
            .filter(|(key, _)| key.ends_with("-api-server"))
            .map(|(key, value)| {
                let api_name = key.trim_end_matches("-api-server").to_string();
                (api_name, value)
            })
            .collect();

        Ok(result)
    }

    pub(super) fn has_spec_changed(&self, api_name: &str, current_hash: &str) -> Result<bool, String> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

        #[derive(QueryableByName)]
        struct HashRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            spec_hash: String,
        }

        let result: Option<HashRow> = diesel::sql_query(
            "SELECT spec_hash FROM generated_api_tools \
             WHERE bot_id = $1 AND api_name = $2 LIMIT 1",
        )
        .bind::<diesel::sql_types::Uuid, _>(self.bot_id)
        .bind::<diesel::sql_types::Text, _>(api_name)
        .get_result(&mut conn)
        .optional()
        .map_err(|e| format!("Failed to check spec hash: {}", e))?;

        match result {
            Some(row) => Ok(row.spec_hash != current_hash),
            None => Ok(true),
        }
    }

    pub(super) fn update_api_record(
        &self,
        api_name: &str,
        spec_url: &str,
        spec_hash: &str,
        tool_count: usize,
    ) -> Result<(), String> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

        let now = chrono::Utc::now();
        let new_id = Uuid::new_v4();

        diesel::sql_query(
            "INSERT INTO generated_api_tools \
             (id, bot_id, api_name, spec_url, spec_hash, tool_count, last_synced_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $7) \
             ON CONFLICT (bot_id, api_name) DO UPDATE SET \
             spec_url = EXCLUDED.spec_url, \
             spec_hash = EXCLUDED.spec_hash, \
             tool_count = EXCLUDED.tool_count, \
             last_synced_at = EXCLUDED.last_synced_at",
        )
        .bind::<diesel::sql_types::Uuid, _>(new_id)
        .bind::<diesel::sql_types::Uuid, _>(self.bot_id)
        .bind::<diesel::sql_types::Text, _>(api_name)
        .bind::<diesel::sql_types::Text, _>(spec_url)
        .bind::<diesel::sql_types::Text, _>(spec_hash)
        .bind::<diesel::sql_types::Integer, _>(tool_count as i32)
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to update API record: {}", e))?;

        Ok(())
    }

    pub(super) fn cleanup_removed_apis(&self, current_apis: &[(String, String)]) -> Result<usize, String> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

        #[derive(QueryableByName)]
        struct ApiRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            api_name: String,
        }

        let existing: Vec<ApiRow> =
            diesel::sql_query("SELECT api_name FROM generated_api_tools WHERE bot_id = $1")
                .bind::<diesel::sql_types::Uuid, _>(self.bot_id)
                .load(&mut conn)
                .map_err(|e| format!("Failed to query existing APIs: {}", e))?;

        let current_names: Vec<&str> = current_apis.iter().map(|(n, _)| n.as_str()).collect();
        let mut removed_count = 0;

        for api in existing {
            if !current_names.contains(&api.api_name.as_str()) {
                diesel::sql_query(
                    "DELETE FROM generated_api_tools WHERE bot_id = $1 AND api_name = $2",
                )
                .bind::<diesel::sql_types::Uuid, _>(self.bot_id)
                .bind::<diesel::sql_types::Text, _>(&api.api_name)
                .execute(&mut conn)
                .ok();

                let api_folder = format!(
                    "{work_path}/{org_id}.gborg/{bot_id}.gbai/.gbdialog/{api_name}",
                    work_path = self.work_path, org_id = current_org_id(), bot_id = self.bot_id, api_name = api.api_name
                );
                if let Err(e) = std::fs::remove_dir_all(&api_folder) {
                    warn!("Failed to remove API folder {}: {}", api_folder, e);
                } else {
                    info!("Removed API folder: {}", api_folder);
                    removed_count += 1;
                }
            }
        }

        Ok(removed_count)
    }

    pub(super) fn calculate_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub(super) fn sanitize_operation_id(id: &str) -> String {
        id.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .to_lowercase()
    }

    pub(super) fn generate_operation_id(method: &str, path: &str) -> String {
        let path_part = path.trim_matches('/').replace(['/', '{', '}'], "_");
        format!("{}_{}", method.to_lowercase(), path_part)
    }

    pub(super) fn sanitize_param_name(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .to_lowercase()
    }

    pub(super) fn map_openapi_type(openapi_type: &str) -> String {
        match openapi_type.to_lowercase().as_str() {
            "integer" | "number" => "number".to_string(),
            "boolean" => "boolean".to_string(),
            "array" => "array".to_string(),
            "object" => "object".to_string(),
            _ => "string".to_string(),
        }
    }

    pub(super) fn value_to_string(value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => serde_json::to_string(value).unwrap_or_default(),
        }
    }

    pub(super) fn escape_description(desc: &str) -> String {
        desc.replace('"', "'").replace('\n', " ").trim().to_string()
    }
}
