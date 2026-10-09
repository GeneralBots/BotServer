use super::*;

pub fn parse_table_definition(
    source: &str,
) -> Result<Vec<TableDefinition>, Box<dyn Error + Send + Sync>> {
    let mut tables = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("TABLE ") {
            let table_def = parse_single_table(&lines, &mut i)?;
            tables.push(table_def);
        } else {
            i += 1;
        }
    }

    Ok(tables)
}
pub(super) fn parse_single_table(
    lines: &[&str],
    index: &mut usize,
) -> Result<TableDefinition, Box<dyn Error + Send + Sync>> {
    let header_line = lines[*index].trim();

    // Parse table-level READ BY and WRITE BY
    let (read_roles, write_roles) = parse_role_attributes(header_line);

    let parts: Vec<&str> = header_line.split_whitespace().collect();

    if parts.len() < 2 {
        return Err(format!(
            "Invalid TABLE syntax at line {}: {}",
            *index + 1,
            header_line
        )
        .into());
    }

    let table_name = parts[1].to_string();

    // Find connection name (ON keyword)
    let mut connection_name = "default".to_string();
    for i in 2..parts.len() {
        if parts[i].eq_ignore_ascii_case("ON") && i + 1 < parts.len() {
            // Check that the next part is not READ or WRITE
            if !parts[i + 1].eq_ignore_ascii_case("READ")
                && !parts[i + 1].eq_ignore_ascii_case("WRITE")
            {
                connection_name = parts[i + 1].to_string();
            }
            break;
        }
    }

    trace!(
        "Parsing TABLE {} ON {} (read_roles: {:?}, write_roles: {:?})",
        table_name,
        connection_name,
        read_roles,
        write_roles
    );

    *index += 1;
    let mut fields = Vec::new();
    let mut field_order = 0;

    while *index < lines.len() {
        let line = lines[*index].trim();

        if line.is_empty()
            || line.starts_with('\'')
            || line.starts_with("REM")
            || line.starts_with("//")
        {
            *index += 1;
            continue;
        }

        if line.eq_ignore_ascii_case("END TABLE") {
            *index += 1;
            break;
        }

        if let Ok(field) = parse_field_definition(line, field_order) {
            fields.push(field);
            field_order += 1;
        } else {
            warn!("Could not parse field definition: {}", line);
        }

        *index += 1;
    }

    Ok(TableDefinition {
        name: table_name,
        connection_name,
        fields,
        read_roles,
        write_roles,
    })
}
pub(super) fn parse_role_attributes(line: &str) -> (Vec<String>, Vec<String>) {
    let mut read_roles = Vec::new();
    let mut write_roles = Vec::new();

    // Find READ BY "..."
    if let Some(read_idx) = line.to_uppercase().find("READ BY") {
        let after_read = &line[read_idx + 7..];
        if let Some(roles_str) = extract_quoted_string(after_read) {
            read_roles = roles_str
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    // Find WRITE BY "..."
    if let Some(write_idx) = line.to_uppercase().find("WRITE BY") {
        let after_write = &line[write_idx + 8..];
        if let Some(roles_str) = extract_quoted_string(after_write) {
            write_roles = roles_str
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    (read_roles, write_roles)
}
pub(super) fn extract_quoted_string(s: &str) -> Option<String> {
    let trimmed = s.trim();
    if let Some(start) = trimmed.find('"') {
        let after_quote = &trimmed[start + 1..];
        if let Some(end) = after_quote.find('"') {
            return Some(after_quote[..end].to_string());
        }
    }
    None
}
pub(super) fn parse_field_definition(
    line: &str,
    order: i32,
) -> Result<FieldDefinition, Box<dyn Error + Send + Sync>> {
    // Parse field-level READ BY and WRITE BY
    let (read_roles, write_roles) = parse_role_attributes(line);

    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.is_empty() {
        return Err("Empty field definition".into());
    }

    // Handle FIELD keyword: "FIELD id AS STRING" -> ["FIELD", "id", "AS", "STRING"]
    // Skip "FIELD" keyword if present
    let start_idx = if parts[0].eq_ignore_ascii_case("FIELD") { 1 } else { 0 };
    let as_idx = parts.iter().position(|p| p.eq_ignore_ascii_case("AS"));

    if parts.len() < start_idx + 1 {
        return Err("Invalid field definition - missing field name".into());
    }

    let field_name = parts[start_idx].to_string();

    // Parse type from "AS type" or use default if no AS keyword
    let mut field_type = if let Some(as_idx) = as_idx {
        if as_idx + 1 < parts.len() {
            parts[as_idx + 1].to_lowercase()
        } else {
            "text".to_string()
        }
    } else if parts.len() > start_idx + 1 {
        parts[start_idx + 1].to_lowercase()
    } else {
        "text".to_string()
    };

    // Handle type parameters like STRING(255)
    let mut length: Option<i32> = None;
    let mut precision: Option<i32> = None;
    if let Some(paren_start) = field_type.find('(') {
        let base_type = field_type[..paren_start].to_lowercase();
        let params = &field_type[paren_start + 1..field_type.len() - 1];
        let param_parts: Vec<&str> = params.split(',').collect();

        if !param_parts.is_empty() {
            length = param_parts[0].trim().parse().ok();
        }
        if param_parts.len() > 1 {
            precision = param_parts[1].trim().parse().ok();
        }
        field_type = base_type;
    }

    let mut is_key = false;
    let mut reference_table: Option<String> = None;

    // Process remaining parts for KEY and REFERENCES keywords
    for i in (start_idx + 1)..parts.len() {
        let part = parts[i].to_lowercase();
        match part.as_str() {
            "key" => is_key = true,
            "as" => {}  // Skip AS keyword - already handled above
            _ if parts
                .get(i - 1)
                .map(|p| p.eq_ignore_ascii_case("references"))
                .unwrap_or(false) => {}
            "references"
                if i + 1 < parts.len() => {
                    reference_table = Some(parts[i + 1].to_string());
                }
            _ => {}
        }
    }

    Ok(FieldDefinition {
        name: field_name,
        field_type,
        length,
        precision,
        is_key,
        is_nullable: !is_key,
        default_value: None,
        reference_table,
        field_order: order,
        read_roles,
        write_roles,
    })
}
pub fn map_type_to_sql(field: &FieldDefinition, driver: &str) -> String {
    let base_type = match field.field_type.as_str() {
        "string" => {
            let len = field.length.unwrap_or(255);
            format!("VARCHAR({})", len)
        }
        "number" | "integer" | "int" => {
            if field.precision.is_some() {
                let len = field.length.unwrap_or(10);
                let prec = field.precision.unwrap_or(2);
                format!("DECIMAL({},{})", len, prec)
            } else if field.length.is_some() {
                "BIGINT".to_string()
            } else {
                "INTEGER".to_string()
            }
        }
        "double" | "float" => {
            if let (Some(len), Some(prec)) = (field.length, field.precision) {
                format!("DECIMAL({},{})", len, prec)
            } else {
                "DOUBLE PRECISION".to_string()
            }
        }
        "date" => "DATE".to_string(),
        "datetime" | "timestamp" => match driver {
            "mysql" | "mariadb" => "DATETIME".to_string(),
            _ => "TIMESTAMP".to_string(),
        },
        "boolean" | "bool" => match driver {
            "mysql" | "mariadb" => "TINYINT(1)".to_string(),
            _ => "BOOLEAN".to_string(),
        },
        "text" => "TEXT".to_string(),
        "guid" | "uuid" => match driver {
            "mysql" | "mariadb" => "CHAR(36)".to_string(),
            _ => "UUID".to_string(),
        },
        _ => format!("VARCHAR({})", field.length.unwrap_or(255)),
    };

    base_type
}
