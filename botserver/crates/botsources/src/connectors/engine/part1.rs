use super::*;

impl ConnectorEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn create_connector(
        conn: &mut PgConnection,
        req: CreateConnectorRequest,
    ) -> Result<ConnectorConfig, String> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sql_query(
            "INSERT INTO connectors (id, bot_id, name, connector_type, description, auth_config, schedule, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Uuid, _>(req.bot_id)
        .bind::<diesel::sql_types::Text, _>(&req.name)
        .bind::<diesel::sql_types::Text, _>(&req.connector_type)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&req.description)
        .bind::<diesel::sql_types::Jsonb, _>(&serde_json::to_value(&req.auth_config).unwrap_or_default())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&req.schedule)
        .bind::<diesel::sql_types::Bool, _>(true)
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .execute(conn)
        .map_err(|e| format!("Failed to create connector: {e}"))?;

        let endpoints = req.endpoints.unwrap_or_default();
        for ep in &endpoints {
            Self::insert_endpoint(conn, id, ep, now)?;
        }

        Self::get_connector(conn, id)
    }

    pub fn get_connector(
        conn: &mut PgConnection,
        id: Uuid,
    ) -> Result<ConnectorConfig, String> {
        let row: ConnectorRow  = sql_query("SELECT * FROM connectors WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(id)
            .get_result(conn)
            .map_err(|e| format!("Connector not found: {e}"))?;

        Self::row_to_config(conn, row)
    }

    pub fn list_connectors(
        conn: &mut PgConnection,
        bot_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ConnectorConfig>, String> {
        let rows: Vec<ConnectorRow> = sql_query(
            "SELECT * FROM connectors WHERE bot_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .bind::<diesel::sql_types::BigInt, _>(limit)
        .bind::<diesel::sql_types::BigInt, _>(offset)
        .get_results(conn)
        .map_err(|e| format!("Failed to list connectors: {e}"))?;

        let mut configs = Vec::new();
        for row in rows {
            configs.push(Self::row_to_config(conn, row)?);
        }
        Ok(configs)
    }

    pub fn update_connector(
        conn: &mut PgConnection,
        id: Uuid,
        req: UpdateConnectorRequest,
    ) -> Result<ConnectorConfig, String> {
        if let Some(ref name) = req.name {
            sql_query("UPDATE connectors SET name = $1 WHERE id = $2")
                .bind::<diesel::sql_types::Text, _>(name)
                .bind::<diesel::sql_types::Uuid, _>(id)
                .execute(conn)
                .map_err(|e| format!("Failed to update name: {e}"))?;
        }
        if let Some(ref desc) = req.description {
            sql_query("UPDATE connectors SET description = $1 WHERE id = $2")
                .bind::<diesel::sql_types::Text, _>(desc)
                .bind::<diesel::sql_types::Uuid, _>(id)
                .execute(conn)
                .map_err(|e| format!("Failed to update description: {e}"))?;
        }
        if let Some(ref auth) = req.auth_config {
            sql_query("UPDATE connectors SET auth_config = $1 WHERE id = $2")
                .bind::<diesel::sql_types::Jsonb, _>(&serde_json::to_value(auth).unwrap_or_default())
                .bind::<diesel::sql_types::Uuid, _>(id)
                .execute(conn)
                .map_err(|e| format!("Failed to update auth: {e}"))?;
        }
        if let Some(ref schedule) = req.schedule {
            sql_query("UPDATE connectors SET schedule = $1 WHERE id = $2")
                .bind::<diesel::sql_types::Text, _>(schedule)
                .bind::<diesel::sql_types::Uuid, _>(id)
                .execute(conn)
                .map_err(|e| format!("Failed to update schedule: {e}"))?;
        }
        if let Some(active) = req.is_active {
            sql_query("UPDATE connectors SET is_active = $1 WHERE id = $2")
                .bind::<diesel::sql_types::Bool, _>(active)
                .bind::<diesel::sql_types::Uuid, _>(id)
                .execute(conn)
                .map_err(|e| format!("Failed to update active status: {e}"))?;
        }

        sql_query("UPDATE connectors SET updated_at = NOW() WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(conn)
            .map_err(|e| format!("Failed to update timestamp: {e}"))?;

        if let Some(ref endpoints) = req.endpoints {
            sql_query("DELETE FROM connector_endpoints WHERE connector_id = $1")
                .bind::<diesel::sql_types::Uuid, _>(id)
                .execute(conn)
                .map_err(|e| format!("Failed to clear endpoints: {e}"))?;

            let now = Utc::now();
            for ep in endpoints {
                Self::insert_endpoint(conn, id, ep, now)?;
            }
        }

        Self::get_connector(conn, id)
    }

    pub fn delete_connector(
        conn: &mut PgConnection,
        id: Uuid,
    ) -> Result<(), String> {
        sql_query("DELETE FROM connector_sync_logs WHERE connector_id = $1")
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(conn)
            .ok();
        sql_query("DELETE FROM connector_endpoints WHERE connector_id = $1")
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(conn)
            .map_err(|e| format!("Failed to delete endpoints: {e}"))?;
        sql_query("DELETE FROM connectors WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(conn)
            .map_err(|e| format!("Failed to delete connector: {e}"))?;
        Ok(())
    }

    pub fn test_connection(
        conn: &mut PgConnection,
        id: Uuid,
    ) -> Result<String, String> {
        let config = Self::get_connector(conn, id)?;
        let client = build_blocking_client(&config.auth_config)
            .map_err(|e| format!("Failed to build client: {e}"))?;

        let test_url = match config.connector_type {
            ConnectorType::Salesforce => Some("/services/data/v52.0/".to_string()),
            ConnectorType::SharePoint => Some("/v1.0/sites?search=".to_string()),
            _ => config.endpoints.first().map(|ep| ep.url.clone()),
        };

        let base = config.auth_config.base_url
            .as_ref()
            .ok_or_else(|| "Base URL not configured".to_string())?;
        let url = format!("{}{}", base.trim_end_matches('/'), test_url.as_deref().unwrap_or("/"));

        let response = client.get(&url).send()
            .map_err(|e| format!("Connection failed: {e}"))?;
        let status = response.status();
        if status.is_success() || status.as_u16() == 401 {
            Ok("Connection successful".to_string())
        } else {
            Err(format!("Connection returned status {status}"))
        }
    }

    pub fn discover_schema(
        conn: &mut PgConnection,
        id: Uuid,
        endpoint_name: Option<String>,
    ) -> Result<Vec<DiscoveredSchema>, String> {
        let config = Self::get_connector(conn, id)?;
        let client = build_blocking_client(&config.auth_config)
            .map_err(|e| format!("Failed to build client: {e}"))?;

        let mut schemas = Vec::new();
        let base_url = config.auth_config.base_url.as_ref()
            .ok_or_else(|| "Base URL not configured".to_string())?;

        for ep in &config.endpoints {
            if let Some(ref filter) = endpoint_name {
                if &ep.name != filter { continue; }
            }

            let url = format!("{}{}", base_url.trim_end_matches('/'), ep.url);
            let schema = match config.connector_type {
                ConnectorType::Salesforce => {
                    let describe_url = url.replace("/query?", "/sobjects/")
                        .split('?').next().unwrap_or(&url).to_string() + "/describe";
                    let resp = client.get(&describe_url).send()
                        .map_err(|e| format!("Schema discovery failed: {e}"))?;
                    let body: serde_json::Value = resp.json().unwrap_or(serde_json::Value::Null);
                    if let Some(fields) = body["fields"].as_array() {
                        DiscoveredSchema {
                            object_name: ep.name.clone(),
                            fields: fields.iter().map(|f| SchemaField {
                                name: f["name"].as_str().unwrap_or("").to_string(),
                                field_type: f["type"].as_str().unwrap_or("string").to_string(),
                                required: f["nillable"].as_bool().map(|n| !n).unwrap_or(false),
                                max_length: f["length"].as_i64().map(|l| l as i32),
                                values: f["picklistValues"].as_array().map(|v| {
                                    v.iter().filter_map(|e| e["value"].as_str().map(String::from)).collect()
                                }),
                            }).collect(),
                            total_count: None,
                        }
                    } else { continue; }
                }
                _ => {
                    let resp = client.get(&url).send()
                        .map_err(|e| format!("Schema discovery failed: {e}"))?;
                    let body: serde_json::Value = resp.json().unwrap_or(serde_json::Value::Null);
                    let first_item = body.as_array().and_then(|a| a.first())
                        .or_else(|| body["data"].as_array().and_then(|a| a.first()))
                        .or_else(|| body["records"].as_array().and_then(|a| a.first()));

                    if let Some(item) = first_item {
                        DiscoveredSchema {
                            object_name: ep.name.clone(),
                            fields: item.as_object().map(|obj| {
                                obj.iter().map(|(k, v)| SchemaField {
                                    name: k.clone(),
                                    field_type: match v {
                                        serde_json::Value::String(_) => "string",
                                        serde_json::Value::Number(_) => "number",
                                        serde_json::Value::Bool(_) => "boolean",
                                        serde_json::Value::Array(_) => "array",
                                        serde_json::Value::Object(_) => "object",
                                        _ => "string",
                                    }.to_string(),
                                    required: false,
                                    max_length: None,
                                    values: None,
                                }).collect()
                            }).unwrap_or_default(),
                            total_count: body["total_size"].as_i64()
                                .or_else(|| body["total"].as_i64())
                                .or_else(|| body["count"].as_i64()),
                        }
                    } else { continue; }
                }
            };
            schemas.push(schema);
        }

        Ok(schemas)
    }
}

impl Default for ConnectorEngine {
    fn default() -> Self {
        Self::new()
    }
}
