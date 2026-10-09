use super::*;

impl BasicCompiler {
    pub fn process_tables_bas(
        &self,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let bot_name = Self::get_bot_name_from_state(&self.state, self.bot_id)?;
        let work_path = botcore::shared::utils::get_work_path();
        let tables_path = format!(
            "{work_path}/{org_id}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/tables.bas",
            org_id = current_org_id()
        );

        if !Path::new(&tables_path).exists() {
            trace!("tables.bas not found for bot {}, skipping", bot_name);
            return Ok(());
        }

        let tables_content = fs::read_to_string(&tables_path)?;

        trace!(
            "Processing tables.bas for bot {}: {}",
            bot_name,
            tables_path
        );

        if let Some(ref cb) = self.callbacks.process_table_definitions {
            if let Some(ref create_runtime) = self.callbacks.create_runtime {
                let runtime = (create_runtime)(Arc::clone(&self.state));
                (cb)(runtime, self.bot_id, &tables_content)?;
            }
        }

        log::info!("Successfully processed tables.bas for bot {}", bot_name);
        Ok(())
    }

    pub(crate) fn get_bot_name_from_state(
        state: &Arc<AppState>,
        bot_id: uuid::Uuid,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut conn = state.conn.get()?;
        use botcore::shared::models::schema::bots::dsl::*;

        bots.filter(id.eq(bot_id))
            .select(name)
            .first::<String>(&mut *conn)
            .map_err(|e| format!("Failed to get bot name: {}", e).into())
    }

    pub(super) fn get_bot_name_by_id(
        &self,
        bot_id: uuid::Uuid,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        use botcore::shared::models::schema::bots::dsl::*;
        use diesel::QueryDsl;

        let mut conn = self
            .state
            .conn
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        let bot_name: String = bots
            .filter(id.eq(&bot_id))
            .select(name)
            .first(&mut conn)
            .map_err(|e| format!("Failed to get bot name: {}", e))?;

        Ok(bot_name)
    }

    pub(super) fn get_columns_from_database_schema(
        &self,
        table_name: &str,
        bot_id: uuid::Uuid,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        use diesel::sql_query;
        use diesel::sql_types::Text;

        #[derive(QueryableByName)]
        struct ColumnRow {
            #[diesel(sql_type = Text)]
            column_name: String,
        }

        let mut conn = self
            .state
            .conn
            .get()
            .map_err(|e| format!("Failed to get DB connection: {}", e))?;

        let query = format!(
            "SELECT column_name FROM information_schema.columns \
             WHERE table_name = '{}' AND table_schema = 'public' \
             ORDER BY ordinal_position",
            table_name
        );

        let columns: Vec<String> = match sql_query(&query).load(&mut conn) {
            Ok(cols) => {
                if cols.is_empty() {
                    log::info!(
                        "Found 0 columns for table '{}' in main database, trying bot database",
                        table_name
                    );
                    let bot_pool = self.state.bot_database_manager.get_bot_pool(bot_id);
                    if let Some(pool) = bot_pool {
                        let mut bot_conn = pool.get().map_err(|e| format!("Bot DB error: {}", e))?;

                        let bot_query = format!(
                            "SELECT column_name FROM information_schema.columns \
                             WHERE table_name = '{}' AND table_schema = 'public' \
                             ORDER BY ordinal_position",
                            table_name
                        );

                        match sql_query(&bot_query).load(&mut *bot_conn) {
                            Ok(bot_cols) => {
                                log::trace!(
                                    "Found {} columns for table '{}' in bot database",
                                    bot_cols.len(),
                                    table_name
                                );
                                bot_cols
                                    .into_iter()
                                    .map(|c: ColumnRow| c.column_name)
                                    .collect()
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to get columns from bot DB for '{}': {}",
                                    table_name,
                                    e
                                );
                                Vec::new()
                            }
                        }
                    } else {
                        log::error!("No bot database available for bot_id: {}", bot_id);
                        Vec::new()
                    }
                } else {
                    log::trace!(
                        "Found {} columns for table '{}' in main database",
                        cols.len(),
                        table_name
                    );
                    cols.into_iter().map(|c: ColumnRow| c.column_name).collect()
                }
            }
            Err(e) => {
                log::info!(
                    "Failed to get columns for table '{}' from main DB: {}",
                    table_name,
                    e
                );

                let bot_pool = self.state.bot_database_manager.get_bot_pool(bot_id);
                if let Some(pool) = bot_pool {
                    let mut bot_conn = pool.get().map_err(|e| format!("Bot DB error: {}", e))?;

                    let bot_query = format!(
                        "SELECT column_name FROM information_schema.columns \
                         WHERE table_name = '{}' AND table_schema = 'public' \
                         ORDER BY ordinal_position",
                        table_name
                    );

                    match sql_query(&bot_query).load(&mut *bot_conn) {
                        Ok(cols) => {
                            log::trace!(
                                "Found {} columns for table '{}' in bot database",
                                cols.len(),
                                table_name
                            );
                            cols.into_iter()
                                .filter(|c: &ColumnRow| c.column_name != "id")
                                .map(|c: ColumnRow| c.column_name)
                                .collect()
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to get columns from bot DB for '{}': {}",
                                table_name,
                                e
                            );
                            Vec::new()
                        }
                    }
                } else {
                    log::error!("No bot database available for bot_id: {}", bot_id);
                    Vec::new()
                }
            }
        };

        Ok(columns)
    }
}
