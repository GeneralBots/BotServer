use super::*;

impl ConnectorEngine {
    pub fn sync_connector(
        conn: &mut PgConnection,
        id: Uuid,
    ) -> Result<SyncResult, String> {
        let config = Self::get_connector(conn, id)?;
        let client = build_blocking_client(&config.auth_config)
            .map_err(|e| format!("Failed to build client: {e}"))?;
        let started_at = Utc::now();
        let mut total_synced = 0i64;
        let mut total_failed = 0i64;
        let mut endpoint_results = Vec::new();

        sql_query("UPDATE connectors SET last_sync_status = 'running' WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(conn).ok();

        let base_url = config.auth_config.base_url.as_ref()
            .ok_or_else(|| "Base URL not configured".to_string())?.clone();

        for ep in &config.endpoints {
            let ep_start = std::time::Instant::now();
            let url = format!("{}{}", base_url.trim_end_matches('/'), &ep.url);

            let ep_result = match client.get(&url).send() {
                Ok(response) => {
                    match response.json::<serde_json::Value>() {
                        Ok(body) => {
                            let records = body.as_array().cloned()
                                .or_else(|| body["data"].as_array().cloned())
                                .or_else(|| body["records"].as_array().cloned())
                                .unwrap_or_default();
                            let count = records.len() as i64;
                            total_synced += count;
                            EndpointSyncResult {
                                endpoint_name: ep.name.clone(),
                                status: SyncStatus::Success,
                                records_synced: count,
                                records_failed: 0,
                                error_message: None,
                            }
                        }
                        Err(e) => {
                            total_failed += 1;
                            EndpointSyncResult {
                                endpoint_name: ep.name.clone(),
                                status: SyncStatus::Failed,
                                records_synced: 0,
                                records_failed: 1,
                                error_message: Some(format!("Failed to parse response: {e}")),
                            }
                        }
                    }
                }
                Err(e) => {
                    total_failed += 1;
                    EndpointSyncResult {
                        endpoint_name: ep.name.clone(),
                        status: SyncStatus::Failed,
                        records_synced: 0,
                        records_failed: 1,
                        error_message: Some(format!("HTTP request failed: {e}")),
                    }
                }
            };

            let now = Utc::now();
            let sync_log_id = Uuid::new_v4();
            sql_query(
                "INSERT INTO connector_sync_logs (id, connector_id, endpoint_id, status, records_synced, records_failed, duration_ms, error_message, started_at, completed_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
            )
            .bind::<diesel::sql_types::Uuid, _>(sync_log_id)
            .bind::<diesel::sql_types::Uuid, _>(id)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(None::<Uuid>)
            .bind::<diesel::sql_types::Text, _>(serde_json::to_string(&ep_result.status).unwrap_or_default())
            .bind::<diesel::sql_types::BigInt, _>(ep_result.records_synced)
            .bind::<diesel::sql_types::BigInt, _>(ep_result.records_failed)
            .bind::<diesel::sql_types::BigInt, _>(ep_start.elapsed().as_millis() as i64)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(ep_result.error_message.as_ref())
            .bind::<diesel::sql_types::Timestamptz, _>(started_at)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(Some(now))
            .execute(conn)
            .map_err(|e| format!("Failed to insert sync log: {e}"))?;

            endpoint_results.push(ep_result);
        }

        let duration_ms = (Utc::now() - started_at).num_milliseconds();
        let overall_status = if total_failed == 0 {
            SyncStatus::Success
        } else if total_synced == 0 {
            SyncStatus::Failed
        } else {
            SyncStatus::Partial
        };

        let now = Utc::now();
        sql_query("UPDATE connectors SET last_sync_at = $1, last_sync_status = $2, error_log = $3, updated_at = $4 WHERE id = $5")
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .bind::<diesel::sql_types::Text, _>(overall_status.to_string())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(
                if total_failed > 0 { Some(format!("{total_failed} endpoint(s) failed")) } else { None }
            )
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .bind::<diesel::sql_types::Uuid, _>(id)
            .execute(conn).ok();

        Ok(SyncResult {
            status: overall_status,
            records_synced: total_synced,
            records_failed: total_failed,
            duration_ms,
            error_message: if total_failed > 0 { Some(format!("{total_failed} endpoint(s) failed")) } else { None },
            endpoint_results,
        })
    }

    pub fn get_sync_logs(
        conn: &mut PgConnection,
        connector_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SyncLog>, String> {
        #[derive(diesel::QueryableByName)]
        struct SyncLogRow {
            #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Uuid)] connector_id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)] endpoint_id: Option<Uuid>,
            #[diesel(sql_type = diesel::sql_types::Text)] status: String,
            #[diesel(sql_type = diesel::sql_types::BigInt)] records_synced: i64,
            #[diesel(sql_type = diesel::sql_types::BigInt)] records_failed: i64,
            #[diesel(sql_type = diesel::sql_types::BigInt)] duration_ms: i64,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] error_message: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)] started_at: chrono::DateTime<Utc>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)] completed_at: Option<chrono::DateTime<Utc>>,
        }

        let rows: Vec<SyncLogRow> = sql_query(
            "SELECT * FROM connector_sync_logs WHERE connector_id = $1 ORDER BY started_at DESC LIMIT $2 OFFSET $3"
        )
        .bind::<diesel::sql_types::Uuid, _>(connector_id)
        .bind::<diesel::sql_types::BigInt, _>(limit)
        .bind::<diesel::sql_types::BigInt, _>(offset)
        .get_results(conn)
        .map_err(|e| format!("Failed to get sync logs: {e}"))?;

        Ok(rows.into_iter().map(|r| SyncLog {
            id: r.id, connector_id: r.connector_id,
            endpoint_id: r.endpoint_id,
            status: SyncStatus::from(r.status.as_str()),
            records_synced: r.records_synced,
            records_failed: r.records_failed,
            duration_ms: r.duration_ms,
            error_message: r.error_message,
            started_at: r.started_at,
            completed_at: r.completed_at,
        }).collect())
    }

    pub fn get_endpoints(
        conn: &mut PgConnection,
        connector_id: Uuid,
    ) -> Result<Vec<EndpointConfig>, String> {
        #[derive(diesel::QueryableByName)]
        struct EndpointRow {
            #[diesel(sql_type = diesel::sql_types::Text)] name: String,
            #[diesel(sql_type = diesel::sql_types::Text)] method: String,
            #[diesel(sql_type = diesel::sql_types::Text)] url: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Jsonb>)] headers: Option<serde_json::Value>,
            #[diesel(sql_type = diesel::sql_types::Text)] sync_direction: String,
            #[diesel(sql_type = diesel::sql_types::Jsonb)] field_mapping: serde_json::Value,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] schedule: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Jsonb>)] pagination: Option<serde_json::Value>,
        }

        let rows: Vec<EndpointRow> = sql_query(
            "SELECT name, method, url, headers, sync_direction, field_mapping, schedule, pagination FROM connector_endpoints WHERE connector_id = $1 ORDER BY created_at"
        )
        .bind::<diesel::sql_types::Uuid, _>(connector_id)
        .get_results(conn)
        .map_err(|e| format!("Failed to get endpoints: {e}"))?;

        Ok(rows.into_iter().map(|r| EndpointConfig {
            name: r.name, method: r.method, url: r.url,
            headers: r.headers,
            auth_type: AuthType::ApiKey,
            sync_direction: serde_json::from_str(&r.sync_direction).unwrap_or(SyncDirection::Pull),
            field_mapping: serde_json::from_value(r.field_mapping).unwrap_or_default(),
            schedule: r.schedule,
            pagination: r.pagination.and_then(|p| serde_json::from_value(p).ok()),
        }).collect())
    }

    pub(super) fn insert_endpoint(
        conn: &mut PgConnection,
        connector_id: Uuid,
        ep: &EndpointConfig,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), String> {
        sql_query(
            "INSERT INTO connector_endpoints (id, connector_id, name, method, url, headers, sync_direction, field_mapping, schedule, pagination, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"
        )
        .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
        .bind::<diesel::sql_types::Uuid, _>(connector_id)
        .bind::<diesel::sql_types::Text, _>(&ep.name)
        .bind::<diesel::sql_types::Text, _>(&ep.method)
        .bind::<diesel::sql_types::Text, _>(&ep.url)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Jsonb>, _>(ep.headers.as_ref())
        .bind::<diesel::sql_types::Text, _>(serde_json::to_string(&ep.sync_direction).unwrap_or_default())
        .bind::<diesel::sql_types::Jsonb, _>(&serde_json::to_value(&ep.field_mapping).unwrap_or_default())
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&ep.schedule)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Jsonb>, _>(ep.pagination.as_ref().map(|p| serde_json::to_value(p).unwrap_or_default()))
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .execute(conn)
        .map_err(|e| format!("Failed to create endpoint: {e}"))?;
        Ok(())
    }

    pub(super) fn row_to_config(
        conn: &mut PgConnection,
        row: ConnectorRow,
    ) -> Result<ConnectorConfig, String> {
        let endpoints = Self::get_endpoints(conn, row.id)?;

        let auth_config: AuthConfig = serde_json::from_value(row.auth_config).unwrap_or(AuthConfig {
            auth_type: AuthType::None, api_key: None, api_key_header: None,
            username: None, password: None, oauth2_client_id: None,
            oauth2_client_secret: None, oauth2_token_url: None,
            oauth2_scopes: None, base_url: None, extra_headers: None,
        });

        Ok(ConnectorConfig {
            id: row.id, bot_id: row.bot_id, name: row.name,
            connector_type: ConnectorType::from(row.connector_type.as_str()),
            description: row.description, auth_config, endpoints,
            schedule: row.schedule, is_active: row.is_active,
            last_sync_at: row.last_sync_at,
            last_sync_status: row.last_sync_status.clone(),
            error_log: row.error_log,
            created_at: row.created_at, updated_at: row.updated_at,
        })
    }
}
