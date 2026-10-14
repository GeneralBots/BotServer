use super::*;

pub fn configure_db_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(ApiUrls::DB_TABLE, get(list_records_handler).post(create_record_handler))
        .route(ApiUrls::DB_TABLE_RECORD, get(get_record_handler).put(update_record_handler).delete(delete_record_handler))
        .route(ApiUrls::DB_TABLE_COUNT, get(count_records_handler))
        .route(ApiUrls::DB_TABLE_SEARCH, post(search_records_handler))
}
pub async fn list_records_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(table): Path<String>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    let table_name = sanitize_identifier(&table);
    let user_roles = user_roles_from_headers(&headers);
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let order_by = params.order_by.as_deref();
    let order_dir = params.order_dir.as_deref();

    let mut conn = match state.conn.get() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botbasic_data::keywords::db_api::part2", None) })),
            )
                .into_response()
        }
    };

    // Check if table actually exists in database (supports dynamic tables from app_generator)
    if !is_table_allowed_with_conn(&mut conn, &table_name) {
        warn!("Table not found in database: {}", table_name);
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Table '{}' not found", table_name) })),
        )
            .into_response();
    }

    // Check table-level read access
    let access_info =
        match check_table_access(&mut conn, &table_name, &user_roles, AccessType::Read) {
            Ok(info) => info,
            Err(e) => {
                warn!(
                    "Access denied to table {} for user {:?}",
                    table_name, user_roles.user_id
                );
                return (StatusCode::FORBIDDEN, Json(json!({ "error": e }))).into_response();
            }
        };

    // Build safe queries using sql_guard
    // The table was confirmed to exist above, so it may be a runtime-created
    // table outside the system safelist; the builders still enforce identifier
    // and injection rules.
    let query = match build_safe_select_query_for_dynamic_table(
        &table_name,
        order_by,
        order_dir,
        limit,
        offset,
    ) {
        Ok(q) => q,
        Err(e) => {
            warn!("Failed to build safe query: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid query parameters" })),
            )
                .into_response();
        }
    };

    let count_query = match build_safe_count_query_for_dynamic_table(&table_name) {
        Ok(q) => q,
        Err(e) => {
            warn!("Failed to build count query: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid table name" })),
            )
                .into_response();
        }
    };

    let rows: Result<Vec<JsonRow>, _> = sql_query(&query).get_results(&mut conn);
    let total: Result<CountResult, _> = sql_query(&count_query).get_result(&mut conn);

    match (rows, total) {
        (Ok(data), Ok(count_result)) => {
            // Filter fields based on user roles
            let filtered_data: Vec<Value> = data
                .into_iter()
                .map(|r| filter_fields_by_role(r.data, &user_roles, &access_info))
                .collect();

            let response = ListResponse {
                data: filtered_data,
                total: count_result.count,
                limit,
                offset,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        (Err(e), _) | (_, Err(e)) => {
            let sanitized = log_and_sanitize(&e, &format!("list_records_{}", table_name), None);
            (StatusCode::INTERNAL_SERVER_ERROR, sanitized).into_response()
        }
    }
}
pub async fn get_record_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((table, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let table_name = sanitize_identifier(&table);
    let user_roles = user_roles_from_headers(&headers);

    let Ok(record_id) = Uuid::parse_str(&id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(RecordResponse {
                success: false,
                data: None,
                message: Some("Invalid UUID format".to_string()),
            }),
        )
            .into_response();
    };

    let Ok(mut conn) = state.conn.get() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(RecordResponse {
                success: false,
                data: None,
                message: Some("Database connection error".to_string()),
            }),
        )
            .into_response();
    };

    // Check if table actually exists in database (supports dynamic tables from app_generator)
    if !is_table_allowed_with_conn(&mut conn, &table_name) {
        warn!("Table not found in database: {}", table_name);
        return (
            StatusCode::NOT_FOUND,
            Json(RecordResponse {
                success: false,
                data: None,
                message: Some(format!("Table '{}' not found", table_name)),
            }),
        )
            .into_response();
    }

    // Check table-level read access
    let access_info =
        match check_table_access(&mut conn, &table_name, &user_roles, AccessType::Read) {
            Ok(info) => info,
            Err(e) => {
                return (
                    StatusCode::FORBIDDEN,
                    Json(RecordResponse {
                        success: false,
                        data: None,
                        message: Some(e),
                    }),
                )
                    .into_response();
            }
        };

    let query = match build_safe_select_by_id_query_for_dynamic_table(&table_name) {
        Ok(q) => q,
        Err(e) => {
            warn!("Failed to build safe query for {}: {}", table_name, e);
            return (
                StatusCode::BAD_REQUEST,
                Json(RecordResponse {
                    success: false,
                    data: None,
                    message: Some("Invalid table name".to_string()),
                }),
            )
                .into_response();
        }
    };

    let row: Result<Option<JsonRow>, _> = sql_query(&query)
        .bind::<diesel::sql_types::Uuid, _>(record_id)
        .get_result(&mut conn)
        .optional();

    match row {
        Ok(Some(r)) => {
            // Filter fields based on user roles
            let filtered_data = filter_fields_by_role(r.data, &user_roles, &access_info);
            (
                StatusCode::OK,
                Json(RecordResponse {
                    success: true,
                    data: Some(filtered_data),
                    message: None,
                }),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(RecordResponse {
                success: false,
                data: None,
                message: Some("Record not found".to_string()),
            }),
        )
            .into_response(),
        Err(e) => {
            let sanitized = log_and_sanitize(&e, &format!("get_record_{}", table_name), None);
            (StatusCode::INTERNAL_SERVER_ERROR, sanitized).into_response()
        }
    }
}
