use super::*;

pub fn process_email_event(
    state: &Arc<dyn BasicRuntime>,
    event: &EmailReceivedEvent,
    script_path: &str,
) -> Result<(), String> {
    info!(
        "Processing email event {} from {} with script {}",
        event.id, event.from_address, script_path
    );

    let mut conn = state.db_pool().get().map_err(|e| e.to_string())?;

    let update_sql = format!(
        "UPDATE email_received_events SET processed = true, processed_at = NOW() WHERE id = '{}'",
        event.id
    );

    diesel::sql_query(&update_sql)
        .execute(&mut *conn)
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn parse_email_path(path: &str) -> Option<(String, Option<String>)> {
    if let Some(rest) = path.strip_prefix("email://") {
        if let Some(slash_pos) = rest.find('/') {
            let email = &rest[..slash_pos];
            let folder = &rest[slash_pos + 1..];
            return Some((email.to_string(), Some(folder.to_string())));
        }
        return Some((rest.to_string(), None));
    }
    None
}

pub fn is_email_path(path: &str) -> bool {
    path.starts_with("email://")
}

pub fn sanitize_email_for_filename(email: &str) -> String {
    email
        .replace('@', "_at_")
        .replace('.', "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>()
        .to_lowercase()
}
