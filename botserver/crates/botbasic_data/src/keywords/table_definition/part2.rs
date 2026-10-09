use super::*;

pub fn generate_create_table_sql(table: &TableDefinition, driver: &str) -> String {
    let mut sql = format!(
        "CREATE TABLE IF NOT EXISTS {} (\n",
        sanitize_identifier(&table.name)
    );

    let mut column_defs = Vec::new();
    let mut primary_keys = Vec::new();

    for field in &table.fields {
        let sql_type = map_type_to_sql(field, driver);
        let mut col_def = format!("    {} {}", sanitize_identifier(&field.name).to_lowercase(), sql_type);

        if field.is_key {
            primary_keys.push(sanitize_identifier(&field.name).to_lowercase());
        }

        if !field.is_nullable {
            col_def.push_str(" NOT NULL");
        }

        if let Some(ref default) = field.default_value {
            use std::fmt::Write;
            let _ = write!(&mut col_def, " DEFAULT {}", default);
        }

        column_defs.push(col_def);
    }

    sql.push_str(&column_defs.join(",\n"));

    if !primary_keys.is_empty() {
        use std::fmt::Write;
        let _ = write!(&mut sql, ",\n    PRIMARY KEY ({})", primary_keys.join(", "));
    }

    sql.push_str("\n)");

    if driver == "mysql" || driver == "mariadb" {
        use std::fmt::Write;
        let _ = write!(
            &mut sql,
            " ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci"
        );
    }

    sql.push(';');
    sql
}
pub fn load_connection_config(
    state: Arc<dyn BasicRuntime>,
    bot_id: Uuid,
    connection_name: &str,
) -> Result<ExternalConnection, Box<dyn Error + Send + Sync>> {
    let prefix = format!("conn-{}-{}-", bot_id, connection_name);

    let server = state.config_value(&format!("{}Server", prefix))
        .ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Missing {prefix}Server in config"),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

    let database = state.config_value(&format!("{}Name", prefix))
        .ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Missing {prefix}Name in config"),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

    let username = state.config_value(&format!("{}Username", prefix))
        .unwrap_or_default();

    let password = state.config_value(&format!("{}Password", prefix))
        .unwrap_or_default();

    let port = state.config_value(&format!("{}Port", prefix))
        .and_then(|p| p.parse().ok());

    let driver = state.config_value(&format!("{}Driver", prefix))
        .unwrap_or_else(|| "postgres".to_string());

    Ok(ExternalConnection {
        name: connection_name.to_string(),
        driver,
        server,
        port,
        database,
        username,
        password,
    })
}
pub fn build_connection_string(conn: &ExternalConnection) -> String {
    let port = conn.port.unwrap_or(match conn.driver.as_str() {
        "mysql" | "mariadb" => 3306,
        "mssql" | "sqlserver" => 1433,
        _ => 5432,
    });

    match conn.driver.as_str() {
        "mysql" | "mariadb" => {
            format!(
                "mysql://{}:{}@{}:{}/{}",
                conn.username, conn.password, conn.server, port, conn.database
            )
        }
        "mssql" | "sqlserver" => {
            format!(
                "mssql://{}:{}@{}:{}/{}",
                conn.username, conn.password, conn.server, port, conn.database
            )
        }
        _ => {
            format!(
                "postgres://{}:{}@{}:{}/{}",
                conn.username, conn.password, conn.server, port, conn.database
            )
        }
    }
}
pub fn store_table_definition(
    conn: &mut diesel::PgConnection,
    bot_id: Uuid,
    table: &TableDefinition,
) -> Result<Uuid, Box<dyn Error + Send + Sync>> {
    // Convert role vectors to semicolon-separated strings for storage
    let read_roles_str: Option<String> = if table.read_roles.is_empty() {
        None
    } else {
        Some(table.read_roles.join(";"))
    };
    let write_roles_str: Option<String> = if table.write_roles.is_empty() {
        None
    } else {
        Some(table.write_roles.join(";"))
    };

    let table_id: Uuid = diesel::sql_query(
        "INSERT INTO dynamic_table_definitions (bot_id, table_name, connection_name, read_roles, write_roles)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (bot_id, table_name, connection_name)
         DO UPDATE SET updated_at = NOW(), read_roles = $4, write_roles = $5
         RETURNING id",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .bind::<Text, _>(&table.name)
    .bind::<Text, _>(&table.connection_name)
    .bind::<diesel::sql_types::Nullable<Text>, _>(&read_roles_str)
    .bind::<diesel::sql_types::Nullable<Text>, _>(&write_roles_str)
    .get_result::<IdResult>(conn)?
    .id;

    diesel::sql_query("DELETE FROM dynamic_table_fields WHERE table_definition_id = $1")
        .bind::<diesel::sql_types::Uuid, _>(table_id)
        .execute(conn)?;

    for field in &table.fields {
        // Convert field role vectors to semicolon-separated strings
        let field_read_roles: Option<String> = if field.read_roles.is_empty() {
            None
        } else {
            Some(field.read_roles.join(";"))
        };
        let field_write_roles: Option<String> = if field.write_roles.is_empty() {
            None
        } else {
            Some(field.write_roles.join(";"))
        };

        diesel::sql_query(
            "INSERT INTO dynamic_table_fields
             (table_definition_id, field_name, field_type, field_length, field_precision,
              is_key, is_nullable, default_value, reference_table, field_order, read_roles, write_roles)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind::<diesel::sql_types::Uuid, _>(table_id)
        .bind::<Text, _>(&field.name)
        .bind::<Text, _>(&field.field_type)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(field.length)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Integer>, _>(field.precision)
        .bind::<diesel::sql_types::Bool, _>(field.is_key)
        .bind::<diesel::sql_types::Bool, _>(field.is_nullable)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&field.default_value)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&field.reference_table)
        .bind::<diesel::sql_types::Integer, _>(field.field_order)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&field_read_roles)
        .bind::<diesel::sql_types::Nullable<Text>, _>(&field_write_roles)
        .execute(conn)?;
    }

    Ok(table_id)
}
