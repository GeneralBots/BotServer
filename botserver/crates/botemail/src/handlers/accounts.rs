use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "mail")]
use crate::imap_auth::{verify_connectivity, ImapAuth};
use crate::models::{
    extract_user_from_session, AppState, EmailAccountBasicRow, EmailError,
};
use crate::schema::user_email_accounts::dsl::{
    created_at, display_name, email, id, imap_port, imap_server, is_active, is_primary,
    smtp_port, smtp_server, user_email_accounts, user_id,
};
use crate::types::{
    ApiResponse, EmailAccountRequest, EmailAccountResponse,
};

fn encrypt_password(password: &str) -> String {
    general_purpose::STANDARD.encode(password.as_bytes())
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Verifies that a mailbox accepts the supplied credentials.
///
/// The IMAP client is blocking and has no connect timeout of its own, so the
/// check runs on a blocking thread under a strict deadline: a request must not
/// hang on an unreachable host. A check that overruns the deadline is reported
/// as a failure while the blocked thread finishes in the background.
#[cfg(feature = "mail")]
async fn validate_mailbox(host: String, port: u16, auth: ImapAuth) -> Result<(), String> {
    let check = tokio::task::spawn_blocking(move || verify_connectivity(&host, port, &auth));
    match tokio::time::timeout(std::time::Duration::from_secs(15), check).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(e)) => Err(format!("The mailbox check could not run: {e}")),
        Err(_) => Err("Timed out while connecting to the mail server".to_string()),
    }
}

pub async fn add_email_account(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(request): Json<EmailAccountRequest>,
) -> Result<Json<ApiResponse<EmailAccountResponse>>, EmailError> {
    let Ok(current_user_id) = extract_user_from_session(&headers) else {
        return Err(EmailError("Authentication required".to_string()));
    };

    let auth_mode = request
        .auth_mode
        .clone()
        .unwrap_or_else(|| "password".to_string());
    if auth_mode != "password" && auth_mode != "oauth2" {
        return Err(EmailError(format!("Unsupported auth_mode '{auth_mode}'")));
    }
    if auth_mode == "oauth2" {
        // An OAuth2 account needs tokens, which only the provider consent flow
        // can supply. Storing one without them would create a mailbox that can
        // never authenticate.
        return Err(EmailError(
            "OAuth2 accounts are connected through the provider consent flow".to_string(),
        ));
    }
    if request.email.trim().is_empty() || request.username.trim().is_empty() {
        return Err(EmailError(
            "Email address and username are required".to_string(),
        ));
    }
    if request.imap_server.trim().is_empty() || request.smtp_server.trim().is_empty() {
        return Err(EmailError(
            "IMAP and SMTP servers are required".to_string(),
        ));
    }

    #[cfg(feature = "mail")]
    {
        let auth = ImapAuth::Password {
            username: request.username.clone(),
            password: request.password.clone(),
        };
        if let Err(e) = validate_mailbox(request.imap_server.clone(), request.imap_port, auth).await
        {
            // The account is rejected rather than stored: an unreachable host
            // would otherwise fail on every background pass while the user sees
            // an empty inbox with no explanation.
            return Err(EmailError(format!(
                "Could not connect to {}: {e}",
                request.imap_server
            )));
        }
    }

    let account_id = Uuid::new_v4();
    let encrypted_password = encrypt_password(&request.password);

    let resp_email = request.email.clone();
    let resp_display_name = request.display_name.clone();
    let resp_imap_server = request.imap_server.clone();
    let resp_imap_port = request.imap_port;
    let resp_smtp_server = request.smtp_server.clone();
    let resp_smtp_port = request.smtp_port;
    let resp_is_primary = request.is_primary;

    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

        if request.is_primary {
            diesel::update(user_email_accounts.filter(user_id.eq(&current_user_id)))
                .set(is_primary.eq(false))
                .execute(&mut db_conn)
                .ok();
        }

        diesel::sql_query(
            "INSERT INTO user_email_accounts
            (id, user_id, email, display_name, imap_server, imap_port, smtp_server, smtp_port, username, password_encrypted, is_primary, is_active, auth_mode)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
        )
            .bind::<diesel::sql_types::Uuid, _>(account_id)
            .bind::<diesel::sql_types::Uuid, _>(current_user_id)
            .bind::<diesel::sql_types::Text, _>(&request.email)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(request.display_name.as_ref())
            .bind::<diesel::sql_types::Text, _>(&request.imap_server)
            .bind::<diesel::sql_types::Integer, _>(i32::from(request.imap_port))
            .bind::<diesel::sql_types::Text, _>(&request.smtp_server)
            .bind::<diesel::sql_types::Integer, _>(i32::from(request.smtp_port))
            .bind::<diesel::sql_types::Text, _>(&request.username)
            .bind::<diesel::sql_types::Text, _>(&encrypted_password)
            .bind::<diesel::sql_types::Bool, _>(request.is_primary)
            .bind::<diesel::sql_types::Bool, _>(true)
            .bind::<diesel::sql_types::Text, _>("password")
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to insert account: {e}"))?;

        Ok::<_, String>(account_id)
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(EmailAccountResponse {
            id: account_id.to_string(),
            email: resp_email,
            display_name: resp_display_name,
            imap_server: resp_imap_server,
            imap_port: resp_imap_port,
            smtp_server: resp_smtp_server,
            smtp_port: resp_smtp_port,
            is_primary: resp_is_primary,
            is_active: true,
            created_at: chrono::Utc::now().to_rfc3339(),
            auth_mode: "password".to_string(),
            last_sync_at: None,
            last_error: None,
        }),
        message: Some("Email account added successfully".to_string()),
    }))
}

pub async fn list_email_accounts_htmx(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let Ok(_user_id) = extract_user_from_session(&headers) else {
        return axum::response::Html(
            r#"<div class="account-item" onclick="document.getElementById('add-account-modal').showModal()">
                <span>+ Add email account</span>
            </div>"#.to_string(),
        );
    };

    let pool = state.pool.clone();
    let accounts: Vec<EmailAccountBasicRow> = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

        diesel::sql_query(
            "SELECT id, email, display_name, is_primary, last_error FROM user_email_accounts WHERE user_id = $1 AND is_active = true ORDER BY is_primary DESC"
        )
            .bind::<diesel::sql_types::Uuid, _>(_user_id)
            .load::<EmailAccountBasicRow>(&mut db_conn)
            .map_err(|e| format!("Query failed: {e}"))
    })
    .await
    .ok()
    .and_then(Result::ok)
    .unwrap_or_default();

    if accounts.is_empty() {
        return axum::response::Html(
            r#"<div class="account-item" onclick="document.getElementById('add-account-modal').showModal()">
                <span>+ Add email account</span>
            </div>"#.to_string(),
        );
    }

    let mut html = String::new();
    for account in accounts {
        let name = account.display_name.clone().unwrap_or_else(|| account.email.clone());
        let primary_badge = if account.is_primary {
            r#"<span class="badge">Primary</span>"#
        } else {
            ""
        };
        // A mailbox whose last sync failed is marked, so the empty list of
        // messages is explained instead of looking like an idle inbox.
        let health_badge = match account.last_error.as_deref() {
            Some(error) => format!(
                r#"<span class="badge badge-error" title="{}">Sync error</span>"#,
                escape_html(error)
            ),
            None => String::new(),
        };
        use std::fmt::Write;
        let _ = write!(
            html,
            r#"<div class="account-item" data-account-id="{}">
                <span>{}</span>
                {}{}
            </div>"#,
            account.id, name, primary_badge, health_badge
        );
    }

    axum::response::Html(html)
}

pub async fn list_email_accounts(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<ApiResponse<Vec<EmailAccountResponse>>>, EmailError> {
    let Ok(current_user_id) = extract_user_from_session(&headers) else {
        return Err(EmailError("Authentication required".to_string()));
    };

    let pool = state.pool.clone();
    let accounts = tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

        let results = user_email_accounts
            .filter(user_id.eq(current_user_id))
            .filter(is_active.eq(true))
            .order((is_primary.desc(), created_at.desc()))
            .select((
                id, email, display_name, imap_server, imap_port,
                smtp_server, smtp_port, is_primary, is_active, created_at,
                crate::schema::user_email_accounts::auth_mode,
                crate::schema::user_email_accounts::last_sync_at,
                crate::schema::user_email_accounts::last_error,
            ))
            .load::<(
                Uuid, String, Option<String>, String, i32,
                String, i32, bool, bool, chrono::DateTime<chrono::Utc>,
                String, Option<chrono::DateTime<chrono::Utc>>, Option<String>,
            )>(&mut db_conn)
            .map_err(|e| format!("Query failed: {e}"))?;

        Ok::<_, String>(results)
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?;
    let accounts = accounts.map_err(EmailError)?;

    let account_list: Vec<EmailAccountResponse> = accounts
        .into_iter()
        .map(|(acc_id, acc_email, acc_display_name, acc_imap_server, acc_imap_port, acc_smtp_server, acc_smtp_port, acc_is_primary, acc_is_active, acc_created_at, acc_auth_mode, acc_last_sync_at, acc_last_error)| {
            EmailAccountResponse {
                id: acc_id.to_string(),
                email: acc_email,
                display_name: acc_display_name,
                imap_server: acc_imap_server,
                imap_port: acc_imap_port as u16,
                smtp_server: acc_smtp_server,
                smtp_port: acc_smtp_port as u16,
                is_primary: acc_is_primary,
                is_active: acc_is_active,
                created_at: acc_created_at.to_rfc3339(),
                auth_mode: acc_auth_mode,
                last_sync_at: acc_last_sync_at.map(|at| at.to_rfc3339()),
                last_error: acc_last_error,
            }
        })
        .collect();

    Ok(Json(ApiResponse {
        success: true,
        data: Some(account_list),
        message: None,
    }))
}

pub async fn delete_email_account(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, EmailError> {
    let account_uuid =
        Uuid::parse_str(&account_id).map_err(|_| EmailError("Invalid account ID".to_string()))?;

    let pool = state.pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

        diesel::sql_query("UPDATE user_email_accounts SET is_active = false WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(account_uuid)
            .execute(&mut db_conn)
            .map_err(|e| format!("Failed to delete account: {e}"))?;

        Ok::<_, String>(())
    })
    .await
    .map_err(|e| EmailError(format!("Task join error: {e}")))?
    .map_err(EmailError)?;

    Ok(Json(ApiResponse {
        success: true,
        data: Some(()),
        message: Some("Email account deleted".to_string()),
    }))
}
