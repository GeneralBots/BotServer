use super::*;

pub(super) fn execute_insert(
    conn: &mut diesel::PgConnection,
    table: &str,
    data: &Dynamic,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let data_map = dynamic_to_map(data);

    let mut columns: Vec<String> = Vec::new();
    let mut values: Vec<String> = Vec::new();

    for (key, value) in &data_map {
        columns.push(sanitize_identifier(key));
        let value_str = value.to_string();
        let converted_value = convert_date_to_iso_format(&value_str);
        if converted_value.trim().is_empty() {
            values.push("NULL".to_string());
        } else {
            values.push(format!("'{}'", sanitize_sql_value(&converted_value)));
        }
    }

    let query = format!(
        "INSERT INTO {} ({}) VALUES ({}) RETURNING id",
        sanitize_identifier(table).to_lowercase(),
        columns.join(", "),
        values.join(", ")
    );

    trace!("Executing INSERT query: {}", query);

    #[derive(QueryableByName)]
    struct InsertResult {
        #[diesel(sql_type = Text)]
        id: String,
    }

    let result: Result<Vec<InsertResult>, _> = sql_query(&query).load(conn);

    match result {
        Ok(rows) => {
            let id = rows.first().map(|r| r.id.clone()).unwrap_or_default();
            Ok(json!({
                "command": "insert",
                "table": table,
                "id": id,
                "success": true
            }))
        }
        Err(e) => {
            log::error!("INSERT SQL error: {}", e);
            Ok(json!({
                "command": "insert",
                "table": table,
                "success": false,
                "error": botsecurity::error_sanitizer::log_and_sanitize_json(&e, "botbasic_data::keywords::data_operations::part4", None)
            }))
        }
    }
}
pub(super) fn execute_update(
    conn: &mut diesel::PgConnection,
    table: &str,
    filter: &str,
    data: &Dynamic,
) -> Result<(i64, Vec<String>), Box<dyn Error + Send + Sync>> {
    let data_map = dynamic_to_map(data);
    let where_clause = parse_filter_clause(filter)?;

    let mut update_sets: Vec<String> = Vec::new();
    for (key, value) in &data_map {
        let value_str = value.to_string();
        let converted_value = convert_date_to_iso_format(&value_str);
        if converted_value.trim().is_empty() {
            update_sets.push(format!("{} = NULL", sanitize_identifier(key)));
        } else {
            update_sets.push(format!(
                "{} = '{}'",
                sanitize_identifier(key),
                sanitize_sql_value(&converted_value)
            ));
        }
    }

    let query = format!(
        "UPDATE {} SET {} WHERE {} RETURNING id",
        sanitize_identifier(table),
        update_sets.join(", "),
        where_clause
    );

    trace!("Executing UPDATE query: {}", query);

    #[derive(QueryableByName)]
    struct UpdateResult {
        #[diesel(sql_type = Text)]
        id: String,
    }

    let rows: Vec<UpdateResult> = sql_query(&query).load(conn).map_err(|e| {
        log::error!("UPDATE SQL error: {}", e);
        e.to_string()
    })?;

    let ids: Vec<String> = rows.into_iter().map(|r| r.id).collect();
    let count = ids.len() as i64;

    Ok((count, ids))
}
pub(super) fn execute_delete(
    conn: &mut diesel::PgConnection,
    table: &str,
    filter: &str,
) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let where_clause = parse_filter_clause(filter)?;

    let query = format!(
        "DELETE FROM {} WHERE {}",
        sanitize_identifier(table),
        where_clause
    );

    trace!("Executing DELETE query: {}", query);

    let result = sql_query(&query).execute(conn).map_err(|e| {
        log::error!("DELETE SQL error: {}", e);
        e.to_string()
    })?;

    Ok(result as i64)
}
pub(super) fn execute_merge(
    conn: &mut diesel::PgConnection,
    table: &str,
    data: &Dynamic,
    key_field: &str,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let array = to_array(data);
    let mut inserted = 0;
    let mut updated = 0;

    for item in array {
        let item_map = dynamic_to_map(&item);

        let key_value = item_map
            .get(key_field)
            .map(|v| v.to_string())
            .unwrap_or_default();

        if key_value.is_empty() {
            continue;
        }

        let check_query = format!(
            "SELECT COUNT(*) as cnt FROM {} WHERE {} = '{}'",
            sanitize_identifier(table),
            sanitize_identifier(key_field),
            sanitize_sql_value(&key_value)
        );

        #[derive(QueryableByName)]
        struct CountResult {
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            cnt: i64,
        }

        let count_result: Result<Vec<CountResult>, _> = sql_query(&check_query).load(conn);
        let exists = count_result
            .map(|r| r.first().map(|c| c.cnt > 0).unwrap_or(false))
            .unwrap_or(false);

        if exists {
            let mut update_sets: Vec<String> = Vec::new();
            for (key, value) in &item_map {
                if key != key_field {
                    update_sets.push(format!(
                        "{} = '{}'",
                        sanitize_identifier(key),
                        sanitize_sql_value(&value.to_string())
                    ));
                }
            }

            if !update_sets.is_empty() {
                let update_query = format!(
                    "UPDATE {} SET {} WHERE {} = '{}'",
                    sanitize_identifier(table),
                    update_sets.join(", "),
                    sanitize_identifier(key_field),
                    sanitize_sql_value(&key_value)
                );
                let _ = sql_query(&update_query).execute(conn);
                updated += 1;
            }
        } else {
            let mut columns: Vec<String> = Vec::new();
            let mut values: Vec<String> = Vec::new();

            for (key, value) in &item_map {
        columns.push(sanitize_identifier(key).to_lowercase());
                values.push(format!("'{}'", sanitize_sql_value(&value.to_string())));
            }

            let insert_query = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                sanitize_identifier(table).to_lowercase(),
                columns.join(", "),
                values.join(", ")
            );
            let _ = sql_query(&insert_query).execute(conn);
            inserted += 1;
        }
    }

    Ok(json!({
        "command": "merge",
        "table": table,
        "key_field": key_field,
        "inserted": inserted,
        "updated": updated
    }))
}
pub(super) fn execute_fill(data: &Dynamic, template: &Dynamic) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    let template_map = dynamic_to_map(template);
    let array = to_array(data);
    let mut results: Array = Array::new();

    for item in array {
        let item_map = dynamic_to_map(&item);
        let mut result_map: Map = Map::new();

        for (template_key, template_value) in &template_map {
            let template_str = template_value.to_string();

            let mut filled_value = template_str.clone();
            for (data_key, data_value) in &item_map {
                let placeholder = format!("{{{{{}}}}}", data_key);
                filled_value = filled_value.replace(&placeholder, &data_value.to_string());
            }

            result_map.insert(template_key.clone().into(), Dynamic::from(filled_value));
        }

        results.push(Dynamic::from(result_map));
    }

    Ok(Dynamic::from(results))
}
pub(super) fn execute_map(data: &Dynamic, mapping: &str) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    let mappings: HashMap<String, String> = mapping
        .split(',')
        .filter_map(|pair| {
            let parts: Vec<&str> = pair.split("->").collect();
            if parts.len() == 2 {
                Some((parts[0].trim().to_string(), parts[1].trim().to_string()))
            } else {
                None
            }
        })
        .collect();

    let array = to_array(data);
    let mut results: Array = Array::new();

    for item in array {
        let item_map = dynamic_to_map(&item);
        let mut result_map: Map = Map::new();

        for (old_key, value) in &item_map {
            let new_key = mappings.get(old_key).unwrap_or(old_key);
            result_map.insert(new_key.clone().into(), value.clone());
        }

        results.push(Dynamic::from(result_map));
    }

    Ok(Dynamic::from(results))
}
