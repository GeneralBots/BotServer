use super::*;

pub(super) fn log_group_activity(
    state: &dyn BasicRuntime,
    group_id: &str,
    action: &str,
    details: &str,
) -> Result<(), String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let activity_id = Uuid::new_v4().to_string();

    let query = diesel::sql_query(
        "INSERT INTO group_activity_log (id, group_id, action, details, timestamp)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind::<diesel::sql_types::Text, _>(&activity_id)
    .bind::<diesel::sql_types::Text, _>(group_id)
    .bind::<diesel::sql_types::Text, _>(action)
    .bind::<diesel::sql_types::Text, _>(details);

    let now = Utc::now();
    let query = query.bind::<diesel::sql_types::Timestamptz, _>(&now);

    query.execute(&mut *conn).map_err(|e| {
        log::error!("Failed to log activity: {}", e);
        format!("Failed to log activity: {}", e)
    })?;

    Ok(())
}

pub(super) fn create_workspace_structure(
    state: &dyn BasicRuntime,
    team_id: &str,
    team_name: &str,
    template: &str,
) -> Result<(), String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let folders = match template {
        "project" => vec![
            "Documents",
            "Meetings",
            "Resources",
            "Deliverables",
            "Archive",
        ],
        "sales" => vec!["Proposals", "Contracts", "Presentations", "CRM", "Reports"],
        "support" => vec![
            "Tickets",
            "Knowledge Base",
            "FAQs",
            "Training",
            "Escalations",
        ],
        _ => vec!["Documents", "Shared", "Archive"],
    };

    let workspace_base = format!(".gbdrive/workspaces/{}", team_name);

    for folder in folders {
        let folder_path = format!("{}/{}", workspace_base, folder);

        let folder_id = Uuid::new_v4().to_string();
        let query = diesel::sql_query(
            "INSERT INTO workspace_folders (id, team_id, path, name, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind::<diesel::sql_types::Text, _>(&folder_id);
        let now = chrono::Utc::now();
        let query = query
            .bind::<diesel::sql_types::Text, _>(team_id)
            .bind::<diesel::sql_types::Text, _>(&folder_path)
            .bind::<diesel::sql_types::Text, _>(folder)
            .bind::<diesel::sql_types::Timestamptz, _>(&now);

        query.execute(&mut *conn).map_err(|e| {
            log::error!("Failed to create workspace folder: {}", e);
            format!("Failed to create workspace folder: {}", e)
        })?;
    }

    trace!("Created workspace structure for team {}", team_name);
    Ok(())
}

pub(super) fn create_team_channel(state: &dyn BasicRuntime, team_id: &str, team_name: &str) -> Result<(), String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let channel_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let query = diesel::sql_query(
        "INSERT INTO communication_channels (id, team_id, name, type, created_at)
         VALUES ($1, $2, $3, 'team_chat', $4)",
    )
    .bind::<diesel::sql_types::Text, _>(&channel_id)
    .bind::<diesel::sql_types::Text, _>(team_id)
    .bind::<diesel::sql_types::Text, _>(team_name)
    .bind::<diesel::sql_types::Timestamptz, _>(&now);

    query.execute(&mut *conn).map_err(|e| {
        log::error!("Failed to create team channel: {}", e);
        format!("Failed to create team channel: {}", e)
    })?;

    trace!("Created communication channel for team {}", team_name);
    Ok(())
}
