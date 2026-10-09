use super::*;

/// Registers the SEARCH keyword with the Rhai engine
///
/// Syntax: SEARCH "table_name", "query", limit
pub fn register_search_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state = &state;
    let connection = state.db_pool().clone();
    let user_roles = UserRoles::from_user_session(&user);

    // SEARCH table, query, limit
if let Err(e) =     engine
        .register_custom_syntax(
            ["SEARCH", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            {
                let conn = connection.clone();
                let roles = user_roles.clone();
                move |context, inputs| {
                    let table_name = context.eval_expression_tree(&inputs[0])?;
                    let query = context.eval_expression_tree(&inputs[1])?;
                    let limit = context.eval_expression_tree(&inputs[2])?;

                    let mut binding = conn.get().map_err(|e| format!("DB error: {e}"))?;
                    let table_str = table_name.to_string();
                    let query_str = query.to_string();
                    let limit_val = limit.as_int().unwrap_or(10) as i32;

                    let access_info = match check_table_access(
                        &mut binding,
                        &table_str,
                        &roles,
                        AccessType::Read,
                    ) {
                        Ok(info) => info,
                        Err(e) => {
                            warn!("SEARCH access denied: {e}");
                            return Err(e.into());
                        }
                    };

                    let (tx, rx) = std::sync::mpsc::channel();
                    let table_str_clone = table_str.clone();
                    let query_str_clone = query_str.clone();
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build();
                        let result = match rt {
                            Ok(rt) => rt.block_on(async {
                                execute_search(&mut binding, &table_str_clone, &query_str_clone, limit_val)
                            })
                            .map_err(|e| format!("Search error: {e}")),
                            Err(_) => Err("Failed to create runtime".into()),
                        };
                        let _ = tx.send(result);
                    });
                    let result = rx.recv().unwrap_or(Err("Failed to receive result".into()))?;

                    if let Some(results) = result.get("results") {
                        let filtered =
                            filter_fields_by_role(results.clone(), &roles, &access_info);
                        let array = to_array(&utils::json_value_to_dynamic(&filtered));
                        Ok(Dynamic::from(array))
                    } else {
                        Ok(Dynamic::from(rhai::Array::new()))
                    }
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    // SEARCH table, query (default limit = 10)
if let Err(e) =     engine
        .register_custom_syntax(["SEARCH", "$expr$", ",", "$expr$"], false, {
            let conn = connection.clone();
            let roles = user_roles.clone();
            move |context, inputs| {
                let table_name = context.eval_expression_tree(&inputs[0])?;
                let query = context.eval_expression_tree(&inputs[1])?;

                let mut binding = conn.get().map_err(|e| format!("DB error: {e}"))?;
                let table_str = table_name.to_string();
                let query_str = query.to_string();

                let access_info = match check_table_access(
                    &mut binding,
                    &table_str,
                    &roles,
                    AccessType::Read,
                ) {
                    Ok(info) => info,
                    Err(e) => {
                        warn!("SEARCH access denied: {e}");
                        return Err(e.into());
                    }
                };

                let (tx, rx) = std::sync::mpsc::channel();
                let table_str_clone = table_str.clone();
                let query_str_clone = query_str.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result = match rt {
                        Ok(rt) => rt.block_on(async { execute_search(&mut binding, &table_str_clone, &query_str_clone, 10) })
                        .map_err(|e| format!("Search error: {e}")),
                        Err(_) => Err("Failed to create runtime".into()),
                    };
                    let _ = tx.send(result);
                });
                let result = rx.recv().unwrap_or(Err("Failed to receive result".into()))?;

                if let Some(results) = result.get("results") {
                    let filtered = filter_fields_by_role(results.clone(), &roles, &access_info);
                    let array = to_array(&utils::json_value_to_dynamic(&filtered));
                    Ok(Dynamic::from(array))
                } else {
                    Ok(Dynamic::from(rhai::Array::new()))
                }
            }
        })
        {
            log::error!("Failed to register the custom syntax: {e}");
        }

    // Register AUTOCOMPLETE function for quick suggestions
    let conn_autocomplete = connection.clone();
    engine.register_fn(
        "AUTOCOMPLETE",
        move |table: String, prefix: String, limit: i64| -> rhai::Array {
            let mut binding = match conn_autocomplete.get() {
                Ok(c) => c,
                Err(_) => return rhai::Array::new(),
            };

            match execute_autocomplete(&mut binding, &table, &prefix, limit as i32) {
                Ok(suggestions) => suggestions
                    .into_iter()
                    .map(Dynamic::from)
                    .collect(),
                Err(_) => rhai::Array::new(),
            }
        },
    );

    // Register lowercase version
    let conn_autocomplete2 = connection.clone();
    engine.register_fn(
        "autocomplete",
        move |table: String, prefix: String, limit: i64| -> rhai::Array {
            let mut binding = match conn_autocomplete2.get() {
                Ok(c) => c,
                Err(_) => return rhai::Array::new(),
            };

            match execute_autocomplete(&mut binding, &table, &prefix, limit as i32) {
                Ok(suggestions) => suggestions
                    .into_iter()
                    .map(Dynamic::from)
                    .collect(),
                Err(_) => rhai::Array::new(),
            }
        },
    );
}
/// Execute full-text search with relevance ranking
pub fn execute_search(
    conn: &mut diesel::PgConnection,
    table_str: &str,
    query_str: &str,
    limit: i32,
) -> Result<Value, String> {
    trace!("Starting execute_search: table={table_str}, query={query_str}, limit={limit}");

    let safe_table = sanitize_identifier(table_str);
    let safe_query = query_str.replace('\'', "''").replace('\\', "\\\\");

    // Get searchable columns from the table
    let searchable_columns = get_searchable_columns(conn, &safe_table)?;

    if searchable_columns.is_empty() {
        warn!("No searchable columns found for table: {safe_table}");
        return Ok(json!({
            "command": "search",
            "table": table_str,
            "query": query_str,
            "results": [],
            "count": 0
        }));
    }

    // Build search expression combining multiple columns
    let search_columns: Vec<String> = searchable_columns
        .iter()
        .map(|col| format!("COALESCE({}::text, '')", col))
        .collect();

    let combined_columns = search_columns.join(" || ' ' || ");

    // Use trigram similarity for fuzzy matching + ILIKE for direct matches
    let query = format!(
        r#"
        SELECT
            row_to_json(t)::text as row_data,
            GREATEST(
                similarity({combined_columns}, $1),
                CASE WHEN {combined_columns} ILIKE '%' || $1 || '%' THEN 0.5 ELSE 0 END
            ) as relevance
        FROM {safe_table} t
        WHERE
            {combined_columns} ILIKE '%' || $1 || '%'
            OR similarity({combined_columns}, $1) > 0.1
        ORDER BY relevance DESC, id
        LIMIT $2
        "#,
        combined_columns = combined_columns,
        safe_table = safe_table
    );

    // Try with trigram extension, fall back to simple ILIKE if not available
    let raw_results: Vec<SearchResultRow> = match diesel::sql_query(&query)
        .bind::<Text, _>(&safe_query)
        .bind::<Integer, _>(limit)
        .load(conn)
    {
        Ok(results) => results,
        Err(e) => {
            trace!("Trigram search failed, falling back to ILIKE: {e}");
            // Fallback to simple ILIKE search
            return execute_simple_search(conn, &safe_table, &safe_query, limit, &searchable_columns);
        }
    };

    let results: Vec<Value> = raw_results
        .into_iter()
        .filter_map(|row| {
            let mut obj: Value = serde_json::from_str(&row.row_data).ok()?;
            if let Value::Object(ref mut map) = obj {
                map.insert("_relevance".to_string(), json!(row.relevance));
            }
            Some(obj)
        })
        .collect();

    trace!("Search returned {} results", results.len());

    Ok(json!({
        "command": "search",
        "table": table_str,
        "query": query_str,
        "results": results,
        "count": results.len()
    }))
}
