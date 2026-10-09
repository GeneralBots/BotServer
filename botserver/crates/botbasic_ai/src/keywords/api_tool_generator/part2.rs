use super::*;

impl ApiToolGenerator {
    pub fn new(state: Arc<dyn BasicRuntime>, bot_id: Uuid, work_path: &str) -> Self {
        Self {
            state,
            bot_id,
            work_path: work_path.to_string(),
        }
    }

    pub async fn sync_all_api_tools(&self) -> Result<SyncResult, String> {
        let api_configs = self.get_api_configs()?;
        let mut result = SyncResult::default();

        let api_configs_for_cleanup = api_configs.clone();

        for (api_name, spec_url) in api_configs {
            info!("Processing API: {} from {}", api_name, spec_url);

            match self.sync_api_tools(&api_name, &spec_url).await {
                Ok(count) => {
                    result.apis_synced += 1;
                    result.tools_generated += count;
                    info!("Generated {} tools for API: {}", count, api_name);
                }
                Err(e) => {
                    result.errors.push(format!("{}: {}", api_name, e));
                    log::error!("Failed to sync API {}: {}", api_name, e);
                }
            }
        }

        let removed = self.cleanup_removed_apis(&api_configs_for_cleanup)?;
        result.tools_removed = removed;

        Ok(result)
    }

    pub async fn sync_api_tools(&self, api_name: &str, spec_url: &str) -> Result<usize, String> {
        let spec_content = self.fetch_spec(spec_url).await?;
        let spec_hash = Self::calculate_hash(&spec_content);

        if !self.has_spec_changed(api_name, &spec_hash)? {
            trace!("API spec unchanged for {}, skipping", api_name);
            return Ok(0);
        }

        let spec: OpenAPISpec = serde_json::from_str(&spec_content)
            .map_err(|e| format!("Failed to parse OpenAPI spec: {}", e))?;

        let endpoints = Self::extract_endpoints(&spec)?;

        let api_folder = format!(
            "{work_path}/{org_id}.gborg/{bot_id}.gbai/.gbdialog/{api_name}",
            work_path = self.work_path, org_id = current_org_id(), bot_id = self.bot_id
        );
        std::fs::create_dir_all(&api_folder)
            .map_err(|e| format!("Failed to create API folder: {}", e))?;

        let mut generated_count = 0;
        for endpoint in &endpoints {
            let bas_content = Self::generate_bas_file(api_name, endpoint)?;
            let file_path = format!("{}/{}.bas", api_folder, endpoint.operation_id);

            std::fs::write(&file_path, &bas_content)
                .map_err(|e| format!("Failed to write .bas file: {}", e))?;

            generated_count += 1;
        }

        self.update_api_record(api_name, spec_url, &spec_hash, generated_count)?;

        Ok(generated_count)
    }

    async fn fetch_spec(&self, spec_url: &str) -> Result<String, String> {
        if spec_url.starts_with("./") || spec_url.starts_with('/') {
            return std::fs::read_to_string(spec_url)
                .map_err(|e| format!("Failed to read local spec file: {}", e));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(spec_url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch spec: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to fetch spec: HTTP {}", response.status()));
        }

        response
            .text()
            .await
            .map_err(|e| format!("Failed to read spec body: {}", e))
    }

    pub(super) fn extract_endpoints(spec: &OpenAPISpec) -> Result<Vec<GeneratedEndpoint>, String> {
        let mut endpoints = Vec::new();

        let base_url = spec
            .servers
            .as_ref()
            .and_then(|s| s.first())
            .map(|s| s.url.clone())
            .unwrap_or_else(|| "http://localhost".to_string());

        for (path, methods) in &spec.paths {
            for (method, operation) in methods {
                let operation_id = match &operation.operation_id {
                    Some(id) => Self::sanitize_operation_id(id),
                    None => Self::generate_operation_id(method, path),
                };

                let description = operation
                    .summary
                    .clone()
                    .or_else(|| operation.description.clone())
                    .unwrap_or_else(|| format!("{} {}", method.to_uppercase(), path));

                let mut parameters = Vec::new();

                if let Some(params) = &operation.parameters {
                    for param in params {
                        parameters.push(Self::convert_parameter(param));
                    }
                }

                if let Some(body) = &operation.request_body {
                    if let Some(content) = &body.content {
                        if let Some(json_content) = content.get("application/json") {
                            if let Some(schema) = &json_content.schema {
                                let body_params = Self::extract_body_parameters(
                                    schema,
                                    body.required.unwrap_or(false),
                                );
                                parameters.extend(body_params);
                            }
                        }
                    }
                }

                endpoints.push(GeneratedEndpoint {
                    operation_id,
                    method: method.to_uppercase(),
                    path: path.clone(),
                    description,
                    parameters,
                    base_url: base_url.clone(),
                });
            }
        }

        Ok(endpoints)
    }

    pub(super) fn convert_parameter(param: &OpenAPIParameter) -> EndpointParameter {
        let param_type = param
            .schema
            .as_ref()
            .and_then(|s| s.schema_type.as_ref())
            .map(|t| Self::map_openapi_type(t.as_str()))
            .unwrap_or_else(|| "string".to_string());

        let example = param
            .example
            .as_ref()
            .or_else(|| param.schema.as_ref().and_then(|s| s.example.as_ref()))
            .map(Self::value_to_string);

        EndpointParameter {
            name: param.name.clone(),
            param_type,
            location: param.location.clone(),
            description: param.description.clone().unwrap_or_default(),
            required: param.required.unwrap_or(false),
            example,
        }
    }

    pub(super) fn extract_body_parameters(schema: &OpenAPISchema, required: bool) -> Vec<EndpointParameter> {
        let mut params = Vec::new();

        if let Some(properties) = &schema.properties {
            let required_fields = schema.required.clone().unwrap_or_default();

            for (name, prop_schema) in properties {
                let param_type = prop_schema
                    .schema_type
                    .clone()
                    .unwrap_or_else(|| "string".to_owned());

                let example = prop_schema.example.as_ref().map(Self::value_to_string);

                params.push(EndpointParameter {
                    name: name.clone(),
                    param_type: Self::map_openapi_type(&param_type),
                    location: "body".to_string(),
                    description: String::new(),
                    required: required && required_fields.contains(name),
                    example,
                });
            }
        }

        params
    }
}
