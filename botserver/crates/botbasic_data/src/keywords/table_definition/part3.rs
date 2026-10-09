use super::*;

pub fn create_table_on_external_db(
    connection_string: &str,
    create_sql: &str,
    driver: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match driver {
        "mysql" | "mariadb" => create_table_mysql(connection_string, create_sql),
        "postgres" | "postgresql" => create_table_postgres(connection_string, create_sql),
        _ => {
            warn!("Unsupported driver: {}, attempting postgres", driver);
            create_table_postgres(connection_string, create_sql)
        }
    }
}
pub(super) fn create_table_mysql(
    _connection_string: &str,
    _sql: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    Err("MySQL support is disabled. Please use PostgreSQL for dynamic tables.".into())
}
pub(super) fn create_table_postgres(
    connection_string: &str,
    sql: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    use diesel::pg::PgConnection;
    use diesel::prelude::*;

    let mut conn = PgConnection::establish(connection_string)?;
    diesel::sql_query(sql).execute(&mut conn)?;
    info!("PostgreSQL table created successfully");
    Ok(())
}
pub fn process_table_definitions(
    state: Arc<dyn BasicRuntime>,
    bot_id: Uuid,
    source: &str,
) -> Result<Vec<TableDefinition>, Box<dyn Error + Send + Sync>> {
    let tables = parse_table_definition(source)?;

    if tables.is_empty() {
        return Ok(tables);
    }

    // Use schema sync for both debug and release builds (non-destructive)
    use crate::keywords::table_migration::sync_bot_tables;

    info!("Running schema migration sync (non-destructive)");

    match sync_bot_tables(state.clone(), bot_id, source) {
        Ok(result) => {
            info!("Schema sync completed: {} created, {} altered, {} columns added",
                result.tables_created, result.tables_altered, result.columns_added);

            // If sync was successful, skip standard table creation
            if result.tables_created > 0 || result.tables_altered > 0 {
                // Store metadata in dynamic_table_definitions for answer mode
                match state.db_pool().get() {
                    Ok(mut conn) => {
                        for table in &tables {
                            if table.connection_name == "default" {
                                if let Err(e) = store_table_definition(&mut conn, bot_id, table) {
                                    error!("Failed to store table definition for {}: {}", table.name, e);
                                }
                            }
                        }
                        info!("Stored {} table definitions in metadata catalog", tables.len());
                    }
                    Err(e) => {
                        error!("Failed to get DB connection for metadata storage: {}", e);
                    }
                }
                return Ok(tables);
            }
        }
        Err(e) => {
            log::error!("Schema sync failed: {}", e);
            // Fall through to standard table creation
        }
    }

    // Standard table creation (for release builds or as fallback)
    for table in &tables {
        info!(
            "Processing TABLE {} ON {}",
            table.name, table.connection_name
        );

        // No need to store table definition - just create the table in the bot's database

        if table.connection_name == "default" {
            let create_sql = generate_create_table_sql(table, "postgres");
            info!("Creating table {} in bot's database (bot_id={})", table.name, bot_id);
            trace!("SQL: {}", create_sql);

            // Create table in the bot's own database using BotDatabaseManager
            if let Err(e) = state.bot_database_manager().create_table_in_bot_database(bot_id, &create_sql) {
                log::error!("Failed to create table {} in bot database: {}", table.name, e);
                // Continue with other tables even if this one fails
            } else {
                info!("Successfully created table {} in bot database", table.name);
            }
        } else {
            match load_connection_config(state.clone(), bot_id, &table.connection_name) {
                Ok(ext_conn) => {
                    let create_sql = generate_create_table_sql(table, &ext_conn.driver);
                    let conn_string = build_connection_string(&ext_conn);

                    info!(
                        "Creating table {} on {} ({})",
                        table.name, table.connection_name, ext_conn.driver
                    );
                    trace!("SQL: {}", create_sql);

                    let driver = ext_conn.driver.clone();
                    if let Err(e) = create_table_on_external_db(&conn_string, &create_sql, &driver)
                    {
                        log::error!("Failed to create table on external DB: {}", e);
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to load connection config for {}: {}",
                        table.connection_name, e
                    );
                }
            }
        }
    }

    // Store metadata in dynamic_table_definitions for answer mode (data/chart)
    match state.db_pool().get() {
        Ok(mut conn) => {
            for table in &tables {
                if table.connection_name == "default" {
                    if let Err(e) = store_table_definition(&mut conn, bot_id, table) {
                        error!("Failed to store table definition for {}: {}", table.name, e);
                    }
                }
            }
            info!("Stored {} table definitions in metadata catalog (fallback)", tables.len());
        }
        Err(e) => {
            error!("Failed to get DB connection for metadata storage: {}", e);
        }
    }

    Ok(tables)
}
pub fn register_table_definition_keywords(
    _state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    _engine: &mut rhai::Engine,
) {
    trace!("TABLE keyword registered (compile-time only)");
}
