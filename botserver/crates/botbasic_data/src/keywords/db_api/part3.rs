use super::*;

pub async fn create_record_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(table): Path<String>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let table_name = sanitize_identifier(&table);
    let user_roles = user_roles_from_headers(&headers);

    let Some(obj) = payload.as_object() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(RecordResponse {
                success: false,
                data: None,
                message: Some("Payload must be a JSON object".to_string()),
            }),
        )
            .into_response();
    };

    let mut columns: Vec<String> = vec!["id".to_string()];
    let mut values: Vec<String> = vec![format!("'{}'", Uuid::new_v4())];

    for (key, value) in obj {
        let col = sanitize_identifier(key);
        if col.is_empty() || col == "id" {
            continue;
        }
        columns.push(col);
        values.push(value_to_sql(value));
    }

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

    let access_info =
        match check_table_access(&mut conn, &table_name, &user_roles, AccessType::Write) {
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

    // Check field-level write access for fields being inserted
    let field_names: Vec<String> = obj
        .keys()
        .map(|k| sanitize_identifier(k))
        .filter(|k| !k.is_empty() && k != "id")
        .collect();
    if let Err(e) = check_field_write_access(&field_names, &user_roles, &access_info) {
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

    let query = format!(
        "INSERT INTO {} ({}) VALUES ({}) RETURNING row_to_json({}.*)::jsonb as data",
        table_name,
        columns.join(", "),
        values.join(", "),
        table_name
    );

    let row: Result<JsonRow, _> = sql_query(&query).get_result(&mut conn);

    match row {
        Ok(r) => {
            info!("Created record in {table_name}");
            (
                StatusCode::CREATED,
                Json(RecordResponse {
                    success: true,
                    data: Some(r.data),
                    message: None,
                }),
            )
                .into_response()
        }
        Err(e) => {
            let sanitized = log_and_sanitize(&e, &format!("create_record_{}", table_name), None);
            (StatusCode::INTERNAL_SERVER_ERROR, sanitized).into_response()
        }
    }
}
pub async fn update_record_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((table, id)): Path<(String, String)>,
    Json(payload): Json<Value>,
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

    let Some(obj) = payload.as_object() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(RecordResponse {
                success: false,
                data: None,
                message: Some("Payload must be a JSON object".to_string()),
            }),
        )
            .into_response();
    };

    let mut set_clauses: Vec<String> = Vec::new();

    for (key, value) in obj {
        let col = sanitize_identifier(key);
        if col.is_empty() || col == "id" {
            continue;
        }
        set_clauses.push(format!("{} = {}", col, value_to_sql(value)));
    }

    if set_clauses.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(RecordResponse {
                success: false,
                data: None,
                message: Some("No valid fields to update".to_string()),
            }),
        )
            .into_response();
    }

    set_clauses.push("updated_at = NOW()".to_string());

    let mut conn = match state.conn.get() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RecordResponse {
                    success: false,
                    data: None,
                    message: Some(format!("Database connection error: {e}")),
                }),
            )
                .into_response()
        }
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

    // Check table-level write access
    let access_info =
        match check_table_access(&mut conn, &table_name, &user_roles, AccessType::Write) {
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

    // Check field-level write access for fields being updated
    let field_names: Vec<String> = obj
        .keys()
        .map(|k| sanitize_identifier(k))
        .filter(|k| !k.is_empty() && k != "id")
        .collect();
    if let Err(e) = check_field_write_access(&field_names, &user_roles, &access_info) {
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

    let query = format!(
        "UPDATE {} SET {} WHERE id = '{}' RETURNING row_to_json({}.*)::jsonb as data",
        table_name,
        set_clauses.join(", "),
        record_id,
        table_name
    );

    let row: Result<JsonRow, _> = sql_query(&query).get_result(&mut conn);

    match row {
        Ok(r) => {
            info!("Updated record in {table_name}: {record_id}");
            (
                StatusCode::OK,
                Json(RecordResponse {
                    success: true,
                    data: Some(r.data),
                    message: None,
                }),
            )
                .into_response()
        }
        Err(e) => {
            let sanitized = log_and_sanitize(&e, &format!("update_record_{}", table_name), None);
            (StatusCode::INTERNAL_SERVER_ERROR, sanitized).into_response()
        }
    }
}
