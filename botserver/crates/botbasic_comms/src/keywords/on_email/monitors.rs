use super::*;

pub fn execute_on_email(
    conn: &mut diesel::PgConnection,
    bot_id: Uuid,
    email_address: &str,
    script_path: &str,
    filter_from: Option<&str>,
    filter_subject: Option<&str>,
) -> Result<Value, String> {
    use botschema::system_automations;

    // branch_id is NOT NULL (migration 6.5.23); resolve it from the bot before
    // inserting so the email trigger is branch-scoped.
    let bot_branch = botschema::system_automation_branch_id(conn, bot_id)
        .map_err(|e| format!("Failed to resolve branch for bot {bot_id}: {e}"))?;

    let new_automation = (
        system_automations::kind.eq(TriggerKind::EmailReceived as i32),
        system_automations::target.eq(email_address),
        system_automations::param.eq(script_path),
        system_automations::bot_id.eq(bot_id),
        system_automations::branch_id.eq(bot_branch),
    );

    let result = diesel::insert_into(system_automations::table)
        .values(&new_automation)
        .execute(conn)
        .map_err(|e| {
            log::error!("SQL execution error: {}", e);
            e.to_string()
        })?;

    let monitor_id = Uuid::new_v4();
    let insert_sql = format!(
        "INSERT INTO email_monitors (id, bot_id, email_address, script_path, filter_from, filter_subject, is_active) \
         VALUES ('{}', '{}', '{}', '{}', {}, {}, true) \
         ON CONFLICT (bot_id, email_address) DO UPDATE SET \
         script_path = EXCLUDED.script_path, \
         filter_from = EXCLUDED.filter_from, \
         filter_subject = EXCLUDED.filter_subject, \
         is_active = true, \
         updated_at = NOW()",
        monitor_id,
        bot_id,
        email_address.replace('\'', "''"),
        script_path.replace('\'', "''"),
        filter_from.map(|f| format!("'{}'", f.replace('\'', "''"))).unwrap_or_else(|| "NULL".to_string()),
        filter_subject.map(|s| format!("'{}'", s.replace('\'', "''"))).unwrap_or_else(|| "NULL".to_string())
    );

    diesel::sql_query(&insert_sql).execute(conn).map_err(|e| {
        log::error!("Failed to insert email monitor: {}", e);
        e.to_string()
    })?;

    Ok(json!({
        "command": "on_email",
        "email_address": email_address,
        "script_path": script_path,
        "filter_from": filter_from,
        "filter_subject": filter_subject,
        "rows_affected": result
    }))
}

pub fn check_email_monitors(
    state: &Arc<dyn BasicRuntime>,
    bot_id: Uuid,
) -> Result<Vec<(EmailReceivedEvent, String)>, String> {
    let mut conn = state.db_pool().get().map_err(|e| e.to_string())?;

    let monitors_sql = format!(
        "SELECT id, bot_id, email_address, script_path, filter_from, filter_subject, last_uid \
         FROM email_monitors WHERE bot_id = '{}' AND is_active = true",
        bot_id
    );

    #[derive(QueryableByName)]
    struct MonitorRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        bot_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        email_address: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        script_path: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        filter_from: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        filter_subject: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::BigInt>)]
        last_uid: Option<i64>,
    }

    let monitors: Vec<MonitorRow> = diesel::sql_query(&monitors_sql)
        .load(&mut *conn)
        .map_err(|e| e.to_string())?;

    let mut events = Vec::new();

    for monitor in monitors {
        trace!(
            "Checking email monitor for {} on bot {} (last_uid: {:?})",
            monitor.email_address,
            monitor.bot_id,
            monitor.last_uid
        );

        let new_events = fetch_new_emails(
            state,
            monitor.id,
            &monitor.email_address,
            monitor.last_uid.unwrap_or(0),
            monitor.filter_from.as_deref(),
            monitor.filter_subject.as_deref(),
        )?;

        for event in new_events {
            events.push((event, monitor.script_path.clone()));
        }
    }

    Ok(events)
}

pub(super) fn fetch_new_emails(
    _state: &Arc<dyn BasicRuntime>,
    monitor_id: Uuid,
    email_address: &str,
    _last_uid: i64,
    filter_from: Option<&str>,
    filter_subject: Option<&str>,
) -> Result<Vec<EmailReceivedEvent>, String> {
    // Simulation: IMAP requires proper server setup
    // Returns simulated email events for testing
    info!(
        "Email monitoring for {}: returning simulated events (requires IMAP setup for real emails)",
        email_address
    );

    let _now = chrono::Utc::now();
    let mut events = Vec::new();

    // Simulated email 1
    let from1 = "sender@example.com";
    let subject1 = "Weekly Report";

    let matches_from = filter_from.map(|f| from1.to_lowercase().contains(&f.to_lowercase())).unwrap_or(true);
    let matches_subject = filter_subject.map(|s| subject1.to_lowercase().contains(&s.to_lowercase())).unwrap_or(true);

    if matches_from && matches_subject {
        events.push(EmailReceivedEvent {
            id: Uuid::new_v4(),
            monitor_id,
            message_uid: 1001,
            message_id: Some(format!("<{}@example.com>", Uuid::new_v4())),
            from_address: from1.to_string(),
            to_addresses: vec![email_address.to_string()],
            subject: Some(subject1.to_string()),
            has_attachments: true,
            attachments: vec![EmailAttachment {
                filename: "report.pdf".to_string(),
                mime_type: "application/pdf".to_string(),
                size: 1024,
            }],
        });
    }

    // Simulated email 2
    let from2 = "notifications@service.com";
    let subject2 = "New notification";

    let matches_from2 = filter_from.map(|f| from2.to_lowercase().contains(&f.to_lowercase())).unwrap_or(true);
    let matches_subject2 = filter_subject.map(|s| subject2.to_lowercase().contains(&s.to_lowercase())).unwrap_or(true);

    if matches_from2 && matches_subject2 {
        events.push(EmailReceivedEvent {
            id: Uuid::new_v4(),
            monitor_id,
            message_uid: 1002,
            message_id: Some(format!("<{}@service.com>", Uuid::new_v4())),
            from_address: from2.to_string(),
            to_addresses: vec![email_address.to_string()],
            subject: Some(subject2.to_string()),
            has_attachments: false,
            attachments: vec![],
        });
    }

    Ok(events)
}

pub(super) fn _decrypt_password(_encrypted: &str) -> Option<String> {
    // Simulation: password decryption placeholder
    None
}
