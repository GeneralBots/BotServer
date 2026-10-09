use super::*;

pub fn execute_add_member(
    state: &dyn BasicRuntime,
    user: &UserSession,
    group_id: &str,
    user_email: &str,
    role: &str,
) -> Result<String, String> {
    let member_id = Uuid::new_v4().to_string();

    let valid_role = validate_role(role);

    let permissions = get_permissions_for_role(&valid_role);

    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let query = diesel::sql_query(
        "INSERT INTO group_members (id, group_id, user_email, role, permissions, added_by, added_at, is_active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, true)
         ON CONFLICT (group_id, user_email)
         DO UPDATE SET role = $4, permissions = $5, updated_at = $7"
    )
    .bind::<diesel::sql_types::Text, _>(&member_id)
    .bind::<diesel::sql_types::Text, _>(group_id)
    .bind::<diesel::sql_types::Text, _>(user_email)
    .bind::<diesel::sql_types::Text, _>(&valid_role)
    .bind::<diesel::sql_types::Jsonb, _>(&permissions);

    let user_id_str = user.user_id.to_string();
    let now = Utc::now();
    let query = query
        .bind::<diesel::sql_types::Text, _>(&user_id_str)
        .bind::<diesel::sql_types::Timestamptz, _>(&now);

    query.execute(&mut *conn).map_err(|e| {
        log::error!("Failed to add member: {}", e);
        format!("Failed to add member: {}", e)
    })?;

    send_member_invitation(state, group_id, user_email, &valid_role)?;

    log_group_activity(state, group_id, "member_added", user_email)?;

    trace!(
        "Added {} to group {} as {} with permissions {:?}",
        user_email,
        group_id,
        valid_role,
        permissions
    );

    Ok(member_id)
}

pub(super) fn execute_create_team(
    state: &dyn BasicRuntime,
    user: &UserSession,
    name: &str,
    members: Vec<String>,
    workspace_template: &str,
) -> Result<String, String> {
    let team_id = Uuid::new_v4().to_string();

    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let query = diesel::sql_query(
        "INSERT INTO groups (id, name, type, template, created_by, created_at, settings)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind::<diesel::sql_types::Text, _>(&team_id)
    .bind::<diesel::sql_types::Text, _>(name)
    .bind::<diesel::sql_types::Text, _>("team")
    .bind::<diesel::sql_types::Text, _>(workspace_template);

    let user_id_str = user.user_id.to_string();
    let now = Utc::now();
    let permissions_json = serde_json::to_value(json!({
        "workspace_enabled": true,
        "chat_enabled": true,
        "file_sharing": true
    }))
    .map_err(|e| format!("Failed to serialize team permissions: {e}"))?;

    let query = query
        .bind::<diesel::sql_types::Text, _>(&user_id_str)
        .bind::<diesel::sql_types::Timestamptz, _>(&now)
        .bind::<diesel::sql_types::Jsonb, _>(&permissions_json);

    query.execute(&mut *conn).map_err(|e| {
        log::error!("Failed to create team: {}", e);
        format!("Failed to create team: {}", e)
    })?;

    execute_add_member(state, user, &team_id, &user.user_id.to_string(), "admin")?;

    for member_email in &members {
        let role = if member_email == &user.user_id.to_string() {
            "admin"
        } else {
            "member"
        };
        execute_add_member(state, user, &team_id, member_email, role)?;
    }

    create_workspace_structure(state, &team_id, name, workspace_template)?;

    create_team_channel(state, &team_id, name)?;

    trace!(
        "Created team '{}' with {} members (ID: {})",
        name,
        members.len(),
        team_id
    );

    Ok(team_id)
}
