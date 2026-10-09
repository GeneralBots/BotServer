use super::*;

impl BasicCompiler {
    pub(crate) fn convert_save_statements(
        &self,
        source: &str,
        bot_id: uuid::Uuid,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut result = String::new();
        let mut save_counter = 0;

        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed.to_uppercase().starts_with("SAVE ") {
                if let Some(converted) = self.convert_save_line(line, bot_id, &mut save_counter)? {
                    result.push_str(&converted);
                    result.push('\n');
                    continue;
                }
            }

            result.push_str(line);
            result.push('\n');
        }

        Ok(result)
    }

    pub(super) fn convert_save_line(
        &self,
        line: &str,
        bot_id: uuid::Uuid,
        save_counter: &mut usize,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let trimmed = line.trim();
        let upper = trimmed.to_uppercase();
        if !upper.starts_with("SAVE ") {
            return Ok(None);
        }

        let content = &trimmed[4..].trim();

        // Handle SAVE ... TO ... WHERE ... pattern
        // Patterns: SAVE variable TO table WHERE id = expr
        //           SAVE (key = val, ...) TO table WHERE id = expr
        if upper.contains(" WHERE ") && upper.contains(" TO ") {
            return self.convert_save_with_where(content, save_counter);
        }

        let parts = self.parse_save_statement(content)?;

        // If the SAVE uses an inline map literal (#{...}) as the data argument,
        // do NOT convert — let the SAVE keyword handler process the map directly.
        // Converting would split the map on commas and produce an empty #{}.
        if parts.len() >= 2 && parts[1].trim_start().starts_with("#{") {
            return Ok(None);
        }

        if parts.len() <= 2 {
            return Ok(None);
        }

        let table_name = &parts[0];
        let table_name = table_name.trim_matches('"');

        log::trace!(
            "Converting SAVE for table: '{}' (original: '{}')",
            table_name,
            &parts[0]
        );

        let column_names = self.get_table_columns_for_save(table_name, bot_id)?;

        let values: Vec<&String> = parts.iter().skip(1).collect();
        let mut map_pairs = Vec::new();

        log::trace!(
            "Matching {} variables to {} columns",
            values.len(),
            column_names.len()
        );

        for value_var in values.iter() {
            let value_lower = value_var.to_lowercase();

            if let Some(column_name) = column_names
                .iter()
                .find(|col| col.to_lowercase() == value_lower)
            {
                map_pairs.push(format!("{}: {}", column_name, value_var));
            } else {
                log::info!("No matching column for variable '{}'", value_var);
            }
        }

        let map_expr = format!("#{{{}}}", map_pairs.join(", "));
        let data_var = format!("__save_data_{}__", save_counter);
        *save_counter += 1;

        let converted = format!(
            "let {} = {}; SAVE \"{}\", {}",
            data_var, map_expr, table_name, data_var
        );

        Ok(Some(converted))
    }

    pub(super) fn parse_save_statement(
        &self,
        content: &str,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut chars = content.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    current.push('"');
                    chars.next();
                }
                '"' => {
                    in_quotes = !in_quotes;
                    current.push('"');
                }
                ',' if !in_quotes => {
                    parts.push(current.trim().to_string());
                    current = String::new();
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }

        Ok(parts)
    }

    /// Convert SAVE ... TO ... WHERE ... patterns to Rhai SAVE syntax
    ///
    /// Handles:
    ///   SAVE (key = val, ...) TO table WHERE id = expr
    ///     → let __save_data_N__ = #{key: val, ...}; SAVE "table", expr, __save_data_N__
    ///
    ///   SAVE variable TO table WHERE id = expr
    ///     → SAVE "table", expr, variable
    pub(super) fn convert_save_with_where(
        &self,
        content: &str,
        save_counter: &mut usize,
    ) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
        let upper = content.to_uppercase();

        let to_pos = upper.find(" TO ").ok_or("No TO found")?;
        let where_pos = upper.find(" WHERE ").ok_or("No WHERE found")?;

        let data_part = content[..to_pos].trim();
        let table_part = content[to_pos + 4..where_pos].trim();
        let condition_part = content[where_pos + 7..].trim();

        let table_name = table_part.trim_matches('"');

        // Extract id expression from condition (supports "id = expr" or "id=expr")
        let cond_upper = condition_part.to_uppercase();
        let id_expr = if cond_upper.starts_with("ID = ") || cond_upper.starts_with("ID=") {
            let eq_pos = condition_part.find('=').unwrap_or(0);
            condition_part[eq_pos + 1..].trim().to_string()
        } else {
            condition_part.to_string()
        };

        if data_part.starts_with('(') && data_part.ends_with(')') {
            // Parenthesized map: (key1 = val1, key2 = val2)
            // Split by commas respecting nested parens
            let inner = &data_part[1..data_part.len() - 1];
            let items = split_by_commas_outside_parens(inner);
            let rhai_items: Vec<String> = items
                .iter()
                .map(|item| {
                    let eq_pos = item.find('=').unwrap_or(0);
                    let key = item[..eq_pos].trim();
                    let val = item[eq_pos + 1..].trim();
                    format!("{}: {}", key, val)
                })
                .collect();

            let data_var = format!("__save_data_{}__", *save_counter);
            *save_counter += 1;

            // Embed id in data map so Rhai's 2-arg SAVE handler can upsert
            // Format: let __save_data_0__ = #{id: id_expr, key: val, ...}; SAVE "table", __save_data_0__
            let all_items = format!("id: {}, {}", id_expr, rhai_items.join(", "));

            Ok(Some(format!(
                "let {} = #{{{}}}; SAVE \"{}\", {}",
                data_var, all_items, table_name, data_var
            )))
        } else {
            // Variable name: SAVE var TO table WHERE id = expr
            // Pass directly to 2-arg SAVE; if var already has an `id` field,
            // handler will upsert, otherwise it inserts
            Ok(Some(format!(
                "SAVE \"{}\", {}",
                table_name, data_part
            )))
        }
    }

    pub(super) fn get_table_columns_for_save(
        &self,
        table_name: &str,
        bot_id: uuid::Uuid,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        if let Ok(columns) = self.get_columns_from_table_definition(table_name, bot_id) {
            if !columns.is_empty() {
                log::trace!(
                    "Using TABLE definition for '{}': {} columns",
                    table_name,
                    columns.len()
                );
                return Ok(columns);
            }
        }

        self.get_columns_from_database_schema(table_name, bot_id)
    }

    pub(super) fn get_columns_from_table_definition(
        &self,
        table_name: &str,
        bot_id: uuid::Uuid,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let bot_name = self.get_bot_name_by_id(bot_id)?;
        let work_path = botcore::shared::utils::get_work_path();
        let tables_path = format!(
            "{work_path}/{org_id}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/tables.bas",
            org_id = current_org_id()
        );

        let tables_content = fs::read_to_string(&tables_path)?;
        let columns = self.parse_table_definition_for_fields(&tables_content, table_name)?;

        Ok(columns)
    }

    pub(super) fn parse_table_definition_for_fields(
        &self,
        content: &str,
        table_name: &str,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let mut columns = Vec::new();
        let mut in_target_table = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("TABLE ") && trimmed.contains(table_name) {
                in_target_table = true;
                continue;
            }

            if in_target_table {
                if trimmed.starts_with("END TABLE") {
                    break;
                }

                if trimmed.starts_with("FIELD ") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let field_name = parts[1].to_string();
                        columns.push(field_name);
                    }
                }
            }
        }

        Ok(columns)
    }
}
