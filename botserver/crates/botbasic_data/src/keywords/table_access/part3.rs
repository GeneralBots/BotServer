use super::*;

pub fn load_table_access_info(
    conn: &mut diesel::PgConnection,
    table_name: &str,
) -> Option<TableAccessInfo> {
    // Query table-level permissions
    let table_result: Result<TableDefRow, _> = sql_query(
        "SELECT table_name, read_roles, write_roles
         FROM dynamic_table_definitions
         WHERE table_name = $1
         LIMIT 1",
    )
    .bind::<Text, _>(table_name)
    .get_result(conn);

    let Ok(table_def) = table_result else {
        trace!(
            "No table definition found for '{table_name}', allowing open access"
        );
        return None;
    };

    let mut info = TableAccessInfo {
        table_name: table_def.table_name,
        read_roles: parse_roles_string(table_def.read_roles.as_ref()),
        write_roles: parse_roles_string(table_def.write_roles.as_ref()),
        field_read_roles: HashMap::new(),
        field_write_roles: HashMap::new(),
    };

    let fields_result: Result<Vec<FieldDefRow>, _> = sql_query(
        "SELECT f.field_name, f.read_roles, f.write_roles
         FROM dynamic_table_fields f
         JOIN dynamic_table_definitions t ON f.table_definition_id = t.id
         WHERE t.table_name = $1",
    )
    .bind::<Text, _>(table_name)
    .get_results(conn);

    if let Ok(fields) = fields_result {
        for field in fields {
            let field_read = parse_roles_string(field.read_roles.as_ref());
            let field_write = parse_roles_string(field.write_roles.as_ref());

            if !field_read.is_empty() {
                info.field_read_roles
                    .insert(field.field_name.clone(), field_read);
            }
            if !field_write.is_empty() {
                info.field_write_roles.insert(field.field_name, field_write);
            }
        }
    }

    trace!(
        "Loaded access info for table '{}': read_roles={:?}, write_roles={:?}, field_restrictions={}",
        info.table_name,
        info.read_roles,
        info.write_roles,
        info.field_read_roles.len() + info.field_write_roles.len()
    );

    Some(info)
}
pub(super) fn parse_roles_string(roles: Option<&String>) -> Vec<String> {
    roles
        .map(|s| {
            s.split(';')
                .map(|r| r.trim().to_string())
                .filter(|r| !r.is_empty())
                .collect()
        })
        .unwrap_or_default()
}
pub fn check_table_access(
    conn: &mut diesel::PgConnection,
    table_name: &str,
    user_roles: &UserRoles,
    access_type: AccessType,
) -> Result<Option<TableAccessInfo>, String> {
    let access_info = load_table_access_info(conn, table_name);

    if let Some(ref info) = access_info {
        let has_access = match access_type {
            AccessType::Read => info.can_read(user_roles),
            AccessType::Write => info.can_write(user_roles),
        };

        if !has_access {
            let action = match access_type {
                AccessType::Read => "read from",
                AccessType::Write => "write to",
            };
            warn!(
                "Access denied: user {:?} cannot {} table '{}'",
                user_roles.user_id, action, table_name
            );
            return Err(format!(
                "Access denied: insufficient permissions to {} table '{}'",
                action, table_name
            ));
        }
    }

    Ok(access_info)
}
pub fn check_field_write_access(
    fields: &[String],
    user_roles: &UserRoles,
    access_info: &Option<TableAccessInfo>,
) -> Result<(), String> {
    let Some(info) = access_info else {
        return Ok(()); // No access info = allow all
    };

    let mut denied_fields = Vec::new();

    for field in fields {
        if !info.can_write_field(field, user_roles) {
            denied_fields.push(field.clone());
        }
    }

    if denied_fields.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Access denied: insufficient permissions to write field(s): {}",
            denied_fields.join(", ")
        ))
    }
}
pub fn filter_fields_by_role(
    data: Value,
    user_roles: &UserRoles,
    access_info: &Option<TableAccessInfo>,
) -> Value {
    let Some(info) = access_info else {
        return data; // No access info = return all fields
    };

    match data {
        Value::Object(mut map) => {
            let restricted = info.get_restricted_read_fields(user_roles);

            for field in restricted {
                trace!("Filtering out field '{}' due to role restriction", field);
                map.remove(&field);
            }

            Value::Object(map)
        }
        Value::Array(arr) => Value::Array(
            arr.into_iter()
                .map(|v| filter_fields_by_role(v, user_roles, access_info))
                .collect(),
        ),
        other => other,
    }
}
pub fn filter_write_fields(
    data: Value,
    user_roles: &UserRoles,
    access_info: &Option<TableAccessInfo>,
) -> (Value, Vec<String>) {
    let Some(info) = access_info else {
        return (data, Vec::new()); // No access info = allow all
    };

    match data {
        Value::Object(mut map) => {
            let restricted = info.get_restricted_write_fields(user_roles);
            let mut removed = Vec::new();

            for field in &restricted {
                if map.contains_key(field) {
                    trace!(
                        "Removing field '{}' from write data due to role restriction",
                        field
                    );
                    map.remove(field);
                    removed.push(field.clone());
                }
            }

            (Value::Object(map), removed)
        }
        other => (other, Vec::new()),
    }
}
/// Get column names for a table from the database schema
pub fn get_table_columns(conn: &mut PgConnection, table_name: &str) -> Vec<String> {
    use diesel::prelude::*;
    use diesel::sql_types::Text;

    // Define a struct for the query result
    #[derive(diesel::QueryableByName)]
    struct ColumnName {
        #[diesel(sql_type = Text)]
        column_name: String,
    }

    // Query information_schema to get column names
    diesel::sql_query(
        "SELECT column_name FROM information_schema.columns WHERE table_name = $1 ORDER BY ordinal_position"
    )
    .bind::<Text, _>(table_name)
    .load::<ColumnName>(conn)
    .unwrap_or_default()
    .into_iter()
    .map(|c| c.column_name)
    .collect()
}
pub fn register_table_keywords(
    _state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut rhai::Engine,
) {
    let user_roles = UserRoles::from_user_session(&user);
    engine.register_fn("has_role", move |role: &str| {
        user_roles.has_role(role)
    });
}
