use super::*;

#[cfg(feature = "mail")]
pub async fn list_folders(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<FolderInfo>>>, EmailError> {
    let account_uuid = Uuid::parse_str(&account_id)
        .map_err(|_| EmailError("Invalid account ID".to_string()))?;

    let (imap_server, imap_port, username, encrypted_password) = {
        let pool = state.pool.clone();
        let account_info = tokio::task::spawn_blocking(move || {
            let mut db_conn = pool.get().map_err(|e| format!("DB connection error: {e}"))?;

            let result: ImapCredentialsRow = diesel::sql_query(
                "SELECT imap_server, imap_port, username, password_encrypted FROM user_email_accounts WHERE id = $1 AND is_active = true"
            )
            .bind::<diesel::sql_types::Uuid, _>(account_uuid)
            .get_result(&mut db_conn)
            .map_err(|e| format!("Account not found: {e}"))?;

            Ok::<_, String>(result)
        })
        .await
        .map_err(|e| EmailError(format!("Task join error: {e}")))?.map_err(EmailError)?;

        (account_info.imap_server, account_info.imap_port,
        account_info.username, account_info.password_encrypted)
    };

    let password = decrypt_password(&encrypted_password).map_err(EmailError)?;
    let client = imap::ClientBuilder::new(imap_server.as_str(), imap_port as u16)
        .connect().map_err(|e| EmailError(format!("Failed to connect to IMAP: {e:?}")))?;
    let mut session = client.login(&username, &password)
        .map_err(|e| EmailError(format!("Login failed: {e:?}")))?;

    let folders = session.list(None, Some("*"))
        .map_err(|e| EmailError(format!("Failed to list folders: {e:?}")))?;

    let folder_list: Vec<FolderInfo> = folders.iter()
        .map(|f| FolderInfo {
            name: f.name().to_string(), path: f.name().to_string(),
            unread_count: 0, total_count: 0,
        })
        .collect();

    session.logout().ok();
    Ok(Json(ApiResponse { success: true, data: Some(folder_list), message: None }))
}
#[cfg(not(feature = "mail"))]
pub async fn list_folders(
    State(_state): State<Arc<AppState>>,
    Path(_account_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<FolderInfo>>>, EmailError> {
    Ok(Json(ApiResponse { success: false, data: Some(Vec::new()), message: Some("Mail feature not enabled".to_string()) }))
}
