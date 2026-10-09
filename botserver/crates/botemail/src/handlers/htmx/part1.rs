use super::*;

#[cfg(feature = "mail")]
pub async fn list_emails_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let folder = params.get("folder").cloned().unwrap_or_else(|| "inbox".to_string());
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return axum::response::Html(r#"<div class="empty-state"><h3>Authentication required</h3><p>Please sign in to view your emails</p></div>"#.to_string()),
    };

    let pool = state.pool.clone();
    let folder_db = match folder.as_str() {
        "sent" => "Sent".to_string(),
        "drafts" => "Drafts".to_string(),
        "trash" => "Trash".to_string(),
        _ => "INBOX".to_string(),
    };
    let emails_result = tokio::task::spawn_blocking(move || {
        use diesel::sql_query;
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {}", e))?;
        sql_query(
            "SELECT m.id::text AS id, m.from_address AS from_name, \
                    m.from_address AS from_email, m.subject, \
                    COALESCE(m.body_text, '') AS preview, \
                    m.received_at::text AS date, m.is_read AS read \
             FROM email_messages m \
             JOIN user_email_accounts a ON a.id = m.account_id \
             WHERE a.user_id = $1 AND m.folder = $2 \
             ORDER BY m.received_at DESC LIMIT 50",
        )
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .bind::<diesel::sql_types::Text, _>(&folder_db)
        .load::<EmailSummary>(&mut db_conn)
        .map_err(|e| format!("Emails query failed: {e}"))
    }).await;

    let emails = match emails_result {
        Ok(Ok(list)) => list,
        Ok(Err(e)) => {
            log::error!("Email list query error: {}", e);
            return axum::response::Html(r#"<div class="empty-state"><h3>Unable to load emails</h3><p>There was an error. Please try again later.</p></div>"#.to_string());
        }
        Err(e) => {
            log::error!("Email list task error: {}", e);
            return axum::response::Html(r#"<div class="empty-state"><h3>Unable to load emails</h3><p>There was an error. Please try again later.</p></div>"#.to_string());
        }
    };

    let mut html = String::new();
    use std::fmt::Write;
    for email in &emails {
        let unread_class = if !email.read { "unread" } else { "" };
        let _ = write!(html, r##"<div class="mail-item {}" hx-get="/api/ui/email/content/{}" hx-target="#mail-content" hx-swap="innerHTML"><input type="checkbox" class="mail-item-checkbox" data-id="{}" onclick="event.stopPropagation()" style="margin-right: 8px; cursor: pointer;" /><div class="mail-header"><span>{}</span><span class="text-sm text-gray">{}</span></div><div class="mail-subject">{}</div><div class="mail-preview">{}</div></div>"##, unread_class, email.id, email.id, email.from_name, email.date, email.subject, email.preview);
    }

    if html.is_empty() {
        html = format!(r#"<div class="empty-state"><h3>No emails in {}</h3><p>This folder is empty</p></div>"#, folder);
    }

    axum::response::Html(html)
}
#[cfg(not(feature = "mail"))]
pub async fn list_emails_htmx(
    State(_state): State<Arc<AppState>>,
    axum::extract::Query(_params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    axum::response::Html(r#"<div class="empty-state"><h3>Mail feature not enabled</h3></div>"#.to_string())
}
#[cfg(feature = "mail")]
pub async fn list_folders_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return axum::response::Html(r#"<div class="nav-item">Please sign in</div>"#.to_string()),
    };

    let pool = state.pool.clone();
    let counts_result = tokio::task::spawn_blocking(move || {
        use diesel::sql_query;
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {}", e))?;
        #[derive(diesel::QueryableByName)]
        struct CountRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            folder: String,
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            count: i64,
        }
        let rows = sql_query(
            "SELECT m.folder AS folder, COUNT(*) AS count \
             FROM email_messages m \
             JOIN user_email_accounts a ON a.id = m.account_id \
             WHERE a.user_id = $1 \
             GROUP BY m.folder",
        )
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .load::<CountRow>(&mut db_conn)
        .map_err(|e| format!("Folder counts query failed: {e}"))?;
        let mut counts = std::collections::HashMap::new();
        for r in rows {
            counts.insert(r.folder, r.count);
        }
        Ok::<_, String>(counts)
    }).await;

    let folder_counts = match counts_result {
        Ok(Ok(c)) => c,
        _ => return axum::response::Html(r#"<div class="nav-item">Error loading folders</div>"#.to_string()),
    };

    let mut html = String::new();
    for (folder_name, count) in &[("inbox", folder_counts.get("INBOX").unwrap_or(&0)), ("sent", folder_counts.get("Sent").unwrap_or(&0)), ("drafts", folder_counts.get("Drafts").unwrap_or(&0)), ("trash", folder_counts.get("Trash").unwrap_or(&0))] {
        let active = if *folder_name == "inbox" { "active" } else { "" };
        let count_badge = if **count > 0 { format!(r#"<span style="margin-left: auto; font-size: 0.875rem; color: #64748b;">{}</span>"#, count) } else { String::new() };
        let display: String = folder_name.chars().next().unwrap_or_default().to_uppercase().collect::<String>() + &folder_name[1..];
        use std::fmt::Write;
        let _ = write!(html, r##"<div class="nav-item {}" hx-get="/api/ui/email/list?folder={}" hx-target="#mail-list" hx-swap="innerHTML"><span>{}</span> {} {}</div>"##, active, folder_name, "", display, count_badge);
    }

    axum::response::Html(html)
}
#[cfg(not(feature = "mail"))]
pub async fn list_folders_htmx(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    axum::response::Html(r#"<div class="nav-item">Mail feature not enabled</div>"#.to_string())
}
pub async fn compose_email_htmx(State(_state): State<Arc<AppState>>) -> Result<impl IntoResponse, EmailError> {
    Ok(axum::response::Html(r##"<div class="mail-content-view"><h2>Compose New Email</h2><form class="compose-form" hx-post="/api/email/send" hx-target="#mail-content" hx-swap="innerHTML"><div class="form-group"><label>To:</label><input type="email" name="to" required></div><div class="form-group"><label>Subject:</label><input type="text" name="subject" required></div><div class="form-group"><label>Message:</label><textarea name="body" rows="10" required></textarea></div><div class="compose-actions"><button type="submit" class="btn-primary">Send</button><button type="button" class="btn-secondary" hx-post="/api/email/draft" hx-include="closest form">Save Draft</button></div></form></div>"##))
}
#[cfg(feature = "mail")]
pub async fn get_email_content_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, EmailError> {
    let user_id = extract_user_from_session(&headers)
        .map_err(|_| EmailError("Authentication required".to_string()))?;

    let msg_id = uuid::Uuid::parse_str(&id)
        .map_err(|_| EmailError("Invalid message id".to_string()))?;

    // The poller stores full messages (body_html/body_text, uid, flags) in
    // email_messages; read the content from the DB scoped to the caller's
    // accounts instead of fetching live from IMAP by a non-UID UUID (the
    // previous path always failed → clicks rendered nothing).
    let pool = state.pool.clone();
    // `id` is used again for the rendered HTML after the closure, so the
    // closure gets its own copy.
    let id_for_content = id.clone();
    let content = tokio::task::spawn_blocking(move || -> Result<EmailContent, String> {
        use diesel::sql_query;
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

        #[derive(diesel::QueryableByName)]
        struct MsgRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            subject: String,
            #[diesel(sql_type = diesel::sql_types::Varchar)]
            from_address: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            to_addresses: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            body_text: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            body_html: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            received_at: chrono::DateTime<chrono::Utc>,
            #[diesel(sql_type = diesel::sql_types::Bool)]
            is_read: bool,
        }
        let row = sql_query(
            "SELECT m.subject, m.from_address, m.to_addresses, \
                    m.body_text, m.body_html, m.received_at, m.is_read \
             FROM email_messages m \
             JOIN user_email_accounts a ON a.id = m.account_id \
             WHERE m.id = $1 AND a.user_id = $2 LIMIT 1",
        )
        .bind::<diesel::sql_types::Uuid, _>(msg_id)
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .get_result::<MsgRow>(&mut db_conn)
        .optional()
        .map_err(|e| format!("Message query failed: {e}"))?
        .ok_or_else(|| "Email not found".to_string())?;

        let body = row.body_html.filter(|h| !h.is_empty())
            .or(row.body_text)
            .unwrap_or_default();
        let to = row.to_addresses.unwrap_or_default();
        Ok(EmailContent {
            id: id_for_content,
            from_name: row.from_address.clone(),
            from_email: row.from_address,
            to,
            subject: row.subject,
            body,
            date: row.received_at.format("%b %d, %Y %H:%M").to_string(),
            read: row.is_read,
        })
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(|e| EmailError(e))?;

    let html = format!(r##"<div class="mail-content-view"><div id="nudges-banner-{id}" class="nudges-banner" data-email-id="{id}"></div><div class="mail-actions"><button hx-get="/api/ui/email/compose?reply_to={id}" hx-target="#mail-content" hx-swap="innerHTML">Reply</button><button hx-get="/api/ui/email/compose?forward={id}" hx-target="#mail-content" hx-swap="innerHTML">Forward</button><details class="snooze-menu"><summary class="mail-action-btn">Snooze</summary><div class="snooze-presets"><button onclick="snoozeEmail('{id}','later-today')">Later today</button><button onclick="snoozeEmail('{id}','tomorrow')">Tomorrow</button><button onclick="snoozeEmail('{id}','this-weekend')">This weekend</button><button onclick="snoozeEmail('{id}','next-week')">Next week</button></div></details><button hx-delete="/api/ui/email/{id}/delete" hx-target="#mail-list" hx-swap="innerHTML" hx-confirm="Delete this email?">Delete</button></div><h2>{subject}</h2><div style="display: flex; align-items: center; gap: 1rem; margin: 1rem 0;"><div><div style="font-weight: 600;">{from_name}</div><div class="text-sm text-gray">to: {to}</div></div><div style="margin-left: auto;" class="text-sm text-gray">{date}</div></div><div class="mail-body">{body}</div><div id="smart-reply-{id}" class="smart-reply-chips" data-email-id="{id}"></div></div>"##,
    id = id,
    subject = content.subject,
    from_name = content.from_name,
    to = content.to,
    date = content.date,
    body = content.body);

    Ok(axum::response::Html(html))
}
#[cfg(not(feature = "mail"))]
pub async fn get_email_content_htmx(
    State(_state): State<Arc<AppState>>,
    Path(_id): Path<String>,
) -> Result<impl IntoResponse, EmailError> {
    Ok(axum::response::Html(r#"<div class="mail-content-view"><p>Mail feature not enabled</p></div>"#.to_string()))
}
#[cfg(feature = "mail")]
pub async fn delete_email_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let user_id = match extract_user_from_session(&headers) {
        Ok(id) => id,
        Err(_) => return axum::response::Html(r#"<div class="empty-state"><h3>Authentication required</h3><p>Please sign in to delete emails</p></div>"#.to_string()),
    };

    let msg_id = match uuid::Uuid::parse_str(&id) {
        Ok(mid) => mid,
        Err(_) => return axum::response::Html(r#"<div class="empty-state"><h3>Invalid message id</h3></div>"#.to_string()),
    };

    // Delete the message from the local store, scoped to the caller's
    // accounts. The previous path moved it to trash via live IMAP with the
    // DB UUID as a sequence number — always failed. Poller re-sync skips
    // nothing here: a later UID range pass may re-add it, which matches the
    // "soft delete in local store" model.
    let pool = state.pool.clone();
    let deleted = tokio::task::spawn_blocking(move || {
        use diesel::sql_query;
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;
        sql_query(
            "DELETE FROM email_messages m \
             USING user_email_accounts a \
             WHERE m.id = $1 AND a.id = m.account_id AND a.user_id = $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(msg_id)
        .bind::<diesel::sql_types::Uuid, _>(user_id)
        .execute(&mut db_conn)
        .map_err(|e| format!("Delete failed: {e}"))
    })
    .await;

    match deleted {
        Ok(Ok(0)) => axum::response::Html(r#"<div class="empty-state"><h3>Email not found</h3></div>"#.to_string()),
        Ok(Ok(_)) => {
            info!("Email {id} deleted from local store");
            axum::response::Html(r#"<div class="success-message"><p>Email deleted</p></div><script>setTimeout(function() { htmx.trigger('#mail-list', 'load'); }, 100);</script>"#.to_string())
        }
        _ => axum::response::Html(r#"<div class="empty-state"><h3>Error deleting email</h3><p>Could not delete email</p></div>"#.to_string()),
    }
}
#[cfg(not(feature = "mail"))]
pub async fn delete_email_htmx(State(_state): State<Arc<AppState>>, Path(_id): Path<String>) -> impl IntoResponse {
    axum::response::Html(r#"<div class="empty-state"><h3>Mail feature not enabled</h3></div>"#.to_string())
}
