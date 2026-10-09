use super::*;

pub async fn delete_record_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((table, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let table_name = sanitize_identifier(&table);
    let user_roles = user_roles_from_headers(&headers);

    let Ok(record_id) = Uuid::parse_str(&id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(DeleteResponse {
                success: false,
                deleted: 0,
                message: Some("Invalid UUID format".to_string()),
            }),
        )
            .into_response();
    };

    let Ok(mut conn) = state.conn.get() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(DeleteResponse {
                success: false,
                deleted: 0,
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
            Json(DeleteResponse {
                success: false,
                deleted: 0,
                message: Some(format!("Table '{}' not found", table_name)),
            }),
        )
            .into_response();
    }

    if let Err(e) = check_table_access(&mut conn, &table_name, &user_roles, AccessType::Write) {
        return (
            StatusCode::FORBIDDEN,
            Json(DeleteResponse {
                success: false,
                deleted: 0,
                message: Some(e),
            }),
        )
            .into_response();
    }

    let query = format!("DELETE FROM {} WHERE id = $1", table_name);

    let deleted: Result<usize, _> = sql_query(&query)
        .bind::<diesel::sql_types::Uuid, _>(record_id)
        .execute(&mut conn);

    match deleted {
        Ok(count) => {
            info!("Deleted {count} record(s) from {table_name}");
            (
                StatusCode::OK,
                Json(DeleteResponse {
                    success: count > 0,
                    deleted: count as i64,
                    message: if count == 0 {
                        Some("Record not found".to_string())
                    } else {
                        None
                    },
                }),
            )
                .into_response()
        }
        Err(e) => {
            let sanitized = log_and_sanitize(&e, &format!("delete_record_{}", table_name), None);
            (StatusCode::INTERNAL_SERVER_ERROR, sanitized).into_response()
        }
    }
}
pub async fn count_records_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(table): Path<String>,
) -> impl IntoResponse {
    let table_name = sanitize_identifier(&table);
    let user_roles = user_roles_from_headers(&headers);

    // Validate table name (basic check - no SQL injection)
    if let Err(e) = validate_table_name(&table_name) {
        warn!("Invalid table name attempted: {} - {}", table_name, e);
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid table name" })),
        )
            .into_response();
    }

    let mut conn = match state.conn.get() {
        Ok(c) => c,
        Err(e) => {
            let sanitized = log_and_sanitize(&e, "count_records_db_connection", None);
            return (StatusCode::INTERNAL_SERVER_ERROR, sanitized).into_response()
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

    // Check table-level read access (count requires read permission)
    if let Err(e) = check_table_access(&mut conn, &table_name, &user_roles, AccessType::Read) {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": e }))).into_response();
    }

    let query = match build_safe_count_query(&table_name) {
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
    let result: Result<CountResult, _> = sql_query(&query).get_result(&mut conn);

    match result {
        Ok(r) => (StatusCode::OK, Json(json!({ "count": r.count }))).into_response(),
        Err(e) => {
            let sanitized = log_and_sanitize(&e, &format!("count_records_{}", table_name), None);
            (StatusCode::INTERNAL_SERVER_ERROR, sanitized).into_response()
        }
    }
}
