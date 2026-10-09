use super::*;

pub async fn get_schema(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SchemaResponse>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;

    let tables: Vec<SchemaTable> = sql_query(
        "SELECT t.table_name::text,
                COALESCE(c.reltuples::bigint, 0)::bigint AS row_count,
                pg_size_pretty(pg_total_relation_size(quote_ident(t.table_name))) AS table_size
         FROM information_schema.tables t
         LEFT JOIN pg_class c ON c.relname = t.table_name
         WHERE t.table_schema = 'public'
           AND t.table_type = 'BASE TABLE'
         ORDER BY t.table_name",
    )
    .get_results(&mut conn)
    .map_err(|e| internal_error(&format!("Failed to query tables: {e}")))?;

    if tables.is_empty() {
        return Ok(Json(SchemaResponse { tables: Vec::new() }));
    }

    let table_names: Vec<String> = tables.iter().map(|t| t.table_name.clone()).collect();
    let table_list: Vec<String> = table_names
        .iter()
        .map(|n| format!("'{}'", n.replace('\'', "''")))
        .collect();

    let columns: Vec<SchemaColumn> = sql_query(format!(
        "SELECT c.table_name::text, c.column_name::text,
                c.data_type::text, c.is_nullable::boolean,
                c.column_default::text
         FROM information_schema.columns c
         WHERE c.table_schema = 'public'
           AND c.table_name IN ({})
         ORDER BY c.table_name, c.ordinal_position",
        table_list.join(",")
    ))
    .get_results(&mut conn)
    .map_err(|e| internal_error(&format!("Failed to query columns: {e}")))?;

    let pk_cols: Vec<PkInfo> = sql_query(format!(
        "SELECT a.attrelid::regclass::text AS table_name,
                a.attname AS column_name
         FROM pg_index i
         JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
         WHERE i.indisprimary
           AND a.attrelid::regclass::text IN ({})",
        table_list.join(",")
    ))
    .get_results(&mut conn)
    .unwrap_or_default();

    let fk_cols: Vec<FkInfo> = sql_query(format!(
        "SELECT kcu.table_name::text, kcu.column_name::text
         FROM information_schema.table_constraints tc
         JOIN information_schema.key_column_usage kcu
           ON tc.constraint_name = kcu.constraint_name
         WHERE tc.constraint_type = 'FOREIGN KEY'
           AND tc.table_schema = 'public'
           AND tc.table_name IN ({})",
        table_list.join(",")
    ))
    .get_results(&mut conn)
    .unwrap_or_default();

    let pk_set: std::collections::HashSet<(String, String)> = pk_cols
        .iter()
        .map(|p| (p.table_name.clone(), p.column_name.clone()))
        .collect();
    let fk_set: std::collections::HashSet<(String, String)> = fk_cols
        .iter()
        .map(|f| (f.table_name.clone(), f.column_name.clone()))
        .collect();

    let mut result_tables = Vec::new();
    for table in &tables {
        let table_columns: Vec<ColumnInfo> = columns
            .iter()
            .filter(|c| c.table_name == table.table_name)
            .map(|c| ColumnInfo {
                name: c.column_name.clone(),
                data_type: c.data_type.clone(),
                nullable: c.is_nullable,
                default_value: c.column_default.clone(),
                is_pk: pk_set.contains(&(table.table_name.clone(), c.column_name.clone())),
                is_fk: fk_set.contains(&(table.table_name.clone(), c.column_name.clone())),
            })
            .collect();

        let column_count = table_columns.len();
        result_tables.push(TableSchema {
            name: table.table_name.clone(),
            columns: table_columns,
            column_count,
            row_count: table.row_count,
            table_size: table.table_size.clone(),
        });
    }

    Ok(Json(SchemaResponse {
        tables: result_tables,
    }))
}
pub async fn get_table_data(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<TableDataResponse>, (StatusCode, Json<serde_json::Value>)> {
    let bot_id = extract_bot_id(&headers)?;
    let safe_name = sanitize_identifier(&name);
    if safe_name != name {
        return Err(error_response("Invalid table name"));
    }

    let mut conn = get_bot_conn_for_env(&state, bot_id, &headers)?;

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params
        .page_size
        .or(params.limit)
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .min(1000);
    let offset = (page - 1) * page_size;

    let count_result: CountResult = sql_query(format!("SELECT COUNT(*)::bigint AS count FROM {safe_name}"))
        .get_result(&mut conn)
        .map_err(|e| internal_error(&format!("Failed to count rows: {e}")))?;

    let columns: Vec<ColName> = sql_query(format!(
        "SELECT column_name::text FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = '{safe_name}'
         ORDER BY ordinal_position"
    ))
    .get_results(&mut conn)
    .map_err(|e| internal_error(&format!("Failed to get columns: {e}")))?;

    let col_names: Vec<String> = columns.iter().map(|c| c.column_name.clone()).collect();

    let pk_column: Option<String> = sql_query(
        "SELECT a.attname AS column_name
         FROM pg_index i
         JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
         WHERE i.indisprimary AND a.attrelid::regclass::text = $1",
    )
    .bind::<diesel::sql_types::Text, _>(&safe_name)
    .get_results::<PkInfo>(&mut conn)
    .ok()
    .and_then(|rows| rows.first().map(|r| r.column_name.clone()));

    if col_names.is_empty() {
        return Ok(Json(TableDataResponse {
            columns: Vec::new(),
            rows: Vec::new(),
            total: count_result.count,
            total_rows: count_result.count,
            page,
            page_size,
            pk_column,
        }));
    }

    let cast_cols: Vec<String> = col_names
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{}::text AS c{}", sanitize_identifier(c), i + 1))
        .collect();

    let mut query = format!("SELECT {} FROM {safe_name}", cast_cols.join(", "));

    if let Some(filter_col) = &params.filter_col {
        let safe_filter_col = sanitize_identifier(filter_col);
        if safe_filter_col == *filter_col && !safe_filter_col.is_empty() {
            if let Some(filter_val) = &params.filter_val {
                let op = match params.filter_op.as_deref() {
                    Some("eq") | None => "=",
                    Some("neq") => "!=",
                    Some("gt") => ">",
                    Some("gte") => ">=",
                    Some("lt") => "<",
                    Some("lte") => "<=",
                    Some("like") => "LIKE",
                    Some("ilike") => "ILIKE",
                    _ => "=",
                };
                let escaped_val = filter_val.replace('\'', "''");
                let like_val = if matches!(op, "LIKE" | "ILIKE") {
                    format!("{escaped_val}%")
                } else {
                    escaped_val
                };
                query.push_str(&format!(" WHERE {safe_filter_col} {op} '{like_val}'"));
            }
        }
    }

    if let Some(sort_col) = &params.sort {
        let safe_sort = sanitize_identifier(sort_col);
        if safe_sort == *sort_col && !safe_sort.is_empty() {
            let order = match params.sort_order.as_deref() {
                Some("desc") | Some("DESC") => "DESC",
                _ => "ASC",
            };
            query.push_str(&format!(" ORDER BY {safe_sort} {order}"));
        }
    }

    query.push_str(&format!(" LIMIT {page_size} OFFSET {offset}"));
    let row_count = col_names.len();
    let raw_text: Vec<Vec<Option<String>>> = match row_count {
        1 => {
            #[derive(QueryableByName)]
            struct R { #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] c1: Option<String> }
            let rows: Vec<R> = sql_query(&query).get_results(&mut conn)
                .map_err(|e| internal_error(&format!("Query failed: {e}")))?;
            rows.into_iter().map(|r| vec![r.c1]).collect()
        }
        2 => {
            #[derive(QueryableByName)]
            struct R { #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] c1: Option<String>, #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] c2: Option<String> }
            let rows: Vec<R> = sql_query(&query).get_results(&mut conn)
                .map_err(|e| internal_error(&format!("Query failed: {e}")))?;
            rows.into_iter().map(|r| vec![r.c1, r.c2]).collect()
        }
        3 => {
            #[derive(QueryableByName)]
            struct R { #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] c1: Option<String>, #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] c2: Option<String>, #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] c3: Option<String> }
            let rows: Vec<R> = sql_query(&query).get_results(&mut conn)
                .map_err(|e| internal_error(&format!("Query failed: {e}")))?;
            rows.into_iter().map(|r| vec![r.c1, r.c2, r.c3]).collect()
        }
        _ => {
            return execute_via_postgres(
                state,
                &bot_id,
                &query,
                page,
                page_size,
                count_result.count,
                pk_column,
                col_names,
            )
            .await;
        }
    };

    let rows: Vec<serde_json::Value> = raw_text
        .into_iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in col_names.iter().enumerate() {
                obj.insert(col.clone(), parse_cell(&row[i]));
            }
            serde_json::Value::Object(obj)
        })
        .collect();

    Ok(Json(TableDataResponse {
        columns: col_names,
        rows,
        total: count_result.count,
        total_rows: count_result.count,
        page,
        page_size,
        pk_column,
    }))
}
pub(super) fn parse_cell(opt: &Option<String>) -> serde_json::Value {
    match opt {
        None => serde_json::Value::Null,
        Some(s) => {
            if s == "true" {
                serde_json::Value::Bool(true)
            } else if s == "false" {
                serde_json::Value::Bool(false)
            } else if let Ok(i) = s.parse::<i64>() {
                serde_json::Value::Number(i.into())
            } else if let Ok(f) = s.parse::<f64>() {
                serde_json::json!(f)
            } else {
                serde_json::Value::String(s.clone())
            }
        }
    }
}
