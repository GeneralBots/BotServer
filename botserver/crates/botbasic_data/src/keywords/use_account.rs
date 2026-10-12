use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use diesel::prelude::*;
use log::info;
use rhai::{Dynamic, Engine};
#[cfg(feature = "mail")]
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(QueryableByName)]
struct AccountResult {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    email: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    provider: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    access_token: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    metadata_json: String,
}

#[derive(QueryableByName, Debug, Clone)]
pub struct ActiveAccountResult {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub account_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub email: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub provider: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub qdrant_collection: String,
}

pub fn register_use_account_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    let state_clone = state;
    let session_clone = user;

    if let Err(e) = engine.register_custom_syntax(
        ["USE", "ACCOUNT", "$expr$"],
        true,
        move |context, inputs| {
            let email = context.eval_expression_tree(&inputs[0])?.to_string();

            info!(
                "USE ACCOUNT keyword executed - Email: {}, Session: {}",
                email, session_clone.id
            );

            let session_id = session_clone.id;
            let bot_id = session_clone.bot_id;
            let user_id = session_clone.user_id;
            let conn = state_clone.db_pool().clone();
            let email_clone = email.clone();

            let result = std::thread::spawn(move || {
                add_account_to_session(conn, session_id, bot_id, user_id, &email_clone)
            })
            .join();

            match result {
                Ok(Ok(_)) => {
                    info!("Account '{}' added to session {}", email, session_clone.id);
                    Ok(Dynamic::UNIT)
                }
                Ok(Err(e)) => {
                    log::error!("Failed to add account '{}': {}", email, e);
                    Err(format!("USE_ACCOUNT failed: {}", e).into())
                }
                Err(e) => {
                    log::error!("Thread panic in USE_ACCOUNT: {:?}", e);
                    Err("USE_ACCOUNT failed: thread panic".into())
                }
            }
        },
    ) {
        log::error!("Failed to register USE ACCOUNT syntax: {e}");
    }
}

fn add_account_to_session(
    conn_pool: botbasic_types::types::DbPool,
    session_id: Uuid,
    bot_id: Uuid,
    user_id: Uuid,
    email: &str,
) -> Result<(), String> {
    let mut conn = conn_pool
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    let account: Option<AccountResult> = diesel::sql_query(
        "SELECT id, email, provider, access_token, metadata_json FROM connected_accounts
        WHERE email = $1 AND (bot_id = $2 OR user_id = $3) AND status = 'active'",
    )
    .bind::<diesel::sql_types::Text, _>(email)
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .get_result(&mut conn)
    .optional()
    .map_err(|e| format!("Failed to query account: {}", e))?;

    let Some(account) = account else {
        return Err(format!(
            "Account '{}' not found or not configured. Add it in Sources app.",
            email
        ));
    };

    let qdrant_collection = format!("account_{}_{}", account.provider, account.id);

    let assoc_id = Uuid::new_v4();
    diesel::sql_query(
        "INSERT INTO session_account_associations
        (id, session_id, bot_id, account_id, email, provider, qdrant_collection, is_active)
        VALUES ($1, $2, $3, $4, $5, $6, $7, true)
        ON CONFLICT (session_id, account_id)
        DO UPDATE SET is_active = true, added_at = NOW()"
    )
    .bind::<diesel::sql_types::Uuid, _>(assoc_id)
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .bind::<diesel::sql_types::Uuid, _>(account.id)
    .bind::<diesel::sql_types::Text, _>(&account.email)
    .bind::<diesel::sql_types::Text, _>(&account.provider)
    .bind::<diesel::sql_types::Text, _>(&qdrant_collection)
    .execute(&mut conn)
    .map_err(|e| format!("Failed to add account association: {}", e))?;

    info!(
        "Added account '{}' ({}) to session {} (collection: {})",
        email, account.provider, session_id, qdrant_collection
    );

    if account.provider == "imap" {
        let email_clone = account.email.clone();
        let vault_path = account.access_token.clone();
        let meta_json = account.metadata_json.clone();
        let acc_id = account.id;
        let bot_id_clone = bot_id;
        let user_id_clone = user_id;
        let qdrant_coll = qdrant_collection.clone();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all().build()
            {
                Ok(r) => r,
                Err(e) => {
                    log::error!("Failed to build tokio runtime for IMAP indexing: {e}");
                    return;
                }
            };
            rt.block_on(index_imap_account_emails(
                &email_clone, &vault_path, &meta_json,
                acc_id, bot_id_clone, user_id_clone, &qdrant_coll,
            ));
        });
    }

    Ok(())
}

#[cfg(feature = "mail")]
async fn index_imap_account_emails(
    email: &str,
    vault_path: &str,
    metadata_json: &str,
    account_id: Uuid,
    bot_id: Uuid,
    user_id: Uuid,
    qdrant_collection: &str,
) {
    let meta: HashMap<String, String> = match serde_json::from_str(metadata_json) {
        Ok(m) => m,
        Err(e) => {
            log::error!("Failed to parse IMAP account metadata: {e}");
            return;
        }
    };

    let imap_server = match meta.get("imap_server") {
        Some(s) => s.clone(),
        None => { log::error!("IMAP account missing imap_server in metadata"); return; }
    };
    let imap_port: u16 = meta.get("imap_port").and_then(|p| p.parse().ok()).unwrap_or(993);
    let username = match meta.get("username") {
        Some(s) => s.clone(),
        None => { log::error!("IMAP account missing username in metadata"); return; }
    };

    let password = match botcoresecrets::manager::SecretsManager::get() {
        Ok(sm) => {
            let data = sm.get_secret(vault_path).await.unwrap_or_default();
            data.get("imap_password").cloned().unwrap_or_default()
        }
        Err(e) => {
            log::error!("Vault not available for IMAP indexing: {e}");
            return;
        }
    };

    if password.is_empty() {
        log::error!("IMAP password not found in Vault at {vault_path}");
        return;
    }

    let qdrant_url = std::env::var("QDRANT_URL")
        .unwrap_or_else(|_| std::env::var("VECTORDB_URL").unwrap_or_else(|_| "http://127.0.0.1:6333".to_string()));

    let qdrant_client = match botqdrant::QdrantClient::from_url(&qdrant_url).build() {
        Ok(c) => c,
        Err(e) => { log::error!("Failed to create Qdrant client: {e}"); return; }
    };

    let collections = qdrant_client.list_collections().await.unwrap_or_default();
    let exists = collections.collections.iter().any(|c| c.name == qdrant_collection);
    if !exists {
        if let Err(e) = qdrant_client.create_collection(qdrant_collection, 1536, "Cosine").await {
            log::error!("Failed to create Qdrant collection {qdrant_collection}: {e}");
            return;
        }
    }

    let embedding_gen = botqdrant::embedding::EmbeddingGenerator::new(
        std::env::var("LLM_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:8081".to_string())
    );

    let imap_result = tokio::task::spawn_blocking(move || {
        let client = match imap::ClientBuilder::new(&imap_server, imap_port).connect() {
            Ok(c) => c,
            Err(e) => { log::error!("IMAP connection failed: {e:?}"); return Vec::new(); }
        };
        let mut session = match client.login(&username, &password) {
            Ok(s) => s,
            Err(e) => { log::error!("IMAP login failed: {e:?}"); return Vec::new(); }
        };

        match session.select("INBOX") {
            Ok(m) => info!("Selected INBOX, {} messages", m.exists),
            Err(e) => { log::error!("IMAP select INBOX failed: {e:?}"); return Vec::new(); }
        }

        let seq = match session.fetch("1:50", "(RFC822)") {
            Ok(s) => s,
            Err(e) => { log::error!("IMAP fetch failed: {e:?}"); return Vec::new(); }
        };

        let mut emails = Vec::new();
        for msg in seq.iter() {
            if let Some(body) = msg.body() {
                if let Ok(parsed) = mailparse::parse_mail(body) {
                    let headers = parsed.get_headers();
                    let subject = headers.get_first_value("Subject").unwrap_or_default();
                    let from = headers.get_first_value("From").unwrap_or_default();
                    let to = headers.get_first_value("To").unwrap_or_default();
                    let body_text = parsed.subparts.iter()
                        .find(|p| p.ctype.mimetype == "text/plain")
                        .and_then(|bp| bp.get_body().ok())
                        .or_else(|| parsed.get_body().ok())
                        .unwrap_or_default();

                    let (from_name, from_email) = if let Some(start) = from.find('<') {
                        if let Some(end) = from.find('>') {
                            (from[..start].trim().trim_matches('"').to_string(), from[start+1..end].to_string())
                        } else { (String::new(), from.clone()) }
                    } else { (String::new(), from.clone()) };

                    emails.push(botqdrant::EmailDocument {
                        id: Uuid::new_v4().to_string(),
                        account_id: account_id.to_string(),
                        from_email,
                        from_name,
                        to_email: to,
                        subject,
                        body_text,
                        date: chrono::Utc::now(),
                        folder: "INBOX".to_string(),
                        has_attachments: false,
                        thread_id: None,
                    });
                }
            }
        }
        let _ = session.logout();
        emails
    }).await.unwrap_or_default();

    info!("Fetched {} emails from IMAP for {}", imap_result.len(), email);

    for chunk in imap_result.chunks(10) {
        for email_doc in chunk {
            let text = format!("From: {} <{}>\nSubject: {}\n\n{}",
                email_doc.from_name, email_doc.from_email, email_doc.subject, email_doc.body_text);
            // Email bodies are arbitrary user text; cut on a char boundary.
            let text = botlib::text::truncate_at_char_boundary(&text, 6000);

            match embedding_gen.generate_text_embedding(text).await {
                Ok(embedding) => {
                    let point = serde_json::json!({
                        "id": email_doc.id,
                        "vector": embedding,
                        "payload": serde_json::to_value(email_doc).unwrap_or_default()
                    });
                    if let Err(e) = qdrant_client.upsert_points(qdrant_collection, vec![point]).await {
                        log::error!("Failed to index email '{}': {e}", email_doc.subject);
                    }
                }
                Err(e) => log::error!("Failed to generate embedding: {e}"),
            }
        }
    }
    info!("Indexed IMAP emails for {} into Qdrant collection {}", email, qdrant_collection);
}

#[cfg(not(feature = "mail"))]
async fn index_imap_account_emails(
    _email: &str,
    _vault_path: &str,
    _metadata_json: &str,
    _account_id: Uuid,
    _bot_id: Uuid,
    _user_id: Uuid,
    _qdrant_collection: &str,
) {
    log::debug!("IMAP email indexing not available without 'mail' feature");
}

pub fn get_active_accounts_for_session(
    conn_pool: &botbasic_types::types::DbPool,
    session_id: Uuid,
) -> Result<Vec<ActiveAccountResult>, String> {
    let mut conn = conn_pool
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    let results: Vec<ActiveAccountResult> = diesel::sql_query(
        "SELECT account_id, email, provider, qdrant_collection
        FROM session_account_associations
        WHERE session_id = $1 AND is_active = true
        ORDER BY added_at DESC",
    )
    .bind::<diesel::sql_types::Uuid, _>(session_id)
    .load(&mut conn)
    .map_err(|e| format!("Failed to get active accounts: {}", e))?;

    Ok(results)
}

pub fn parse_account_path(path: &str) -> Option<(String, String)> {
    if let Some(rest) = path.strip_prefix("account://") {
        if let Some(slash_pos) = rest.find('/') {
            let email = &rest[..slash_pos];
            let file_path = &rest[slash_pos + 1..];
            return Some((email.to_string(), file_path.to_string()));
        }
    }
    None
}

pub fn is_account_path(path: &str) -> bool {
    path.starts_with("account://")
}

pub async fn get_account_credentials(
    conn_pool: &botbasic_types::types::DbPool,
    email: &str,
    bot_id: Uuid,
) -> Result<AccountCredentials, String> {
    let mut conn = conn_pool
        .get()
        .map_err(|e| format!("Failed to get DB connection: {}", e))?;

    #[derive(QueryableByName)]
    struct CredResult {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)]
        provider: String,
        #[diesel(sql_type = diesel::sql_types::Text)]
        access_token: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        refresh_token: Option<String>,
    }

    let creds: CredResult = diesel::sql_query(
        "SELECT id, provider, access_token, refresh_token
        FROM connected_accounts
        WHERE email = $1 AND bot_id = $2 AND status = 'active'",
    )
    .bind::<diesel::sql_types::Text, _>(email)
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .get_result(&mut conn)
    .map_err(|e| format!("Account not found: {}", e))?;

    Ok(AccountCredentials {
        account_id: creds.id,
        provider: creds.provider,
        access_token: creds.access_token,
        refresh_token: creds.refresh_token,
    })
}

#[derive(Debug, Clone)]
pub struct AccountCredentials {
    pub account_id: Uuid,
    pub provider: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_account_path() {
        let result = parse_account_path("account://user@gmail.com/Documents/file.pdf");
        assert!(result.is_some());
        let (email, path) = result.unwrap();
        assert_eq!(email, "user@gmail.com");
        assert_eq!(path, "Documents/file.pdf");
    }

    #[test]
    fn test_parse_account_path_invalid() {
        assert!(parse_account_path("local/file.pdf").is_none());
        assert!(parse_account_path("/absolute/path").is_none());
    }

    #[test]
    fn test_is_account_path() {
        assert!(is_account_path("account://user@gmail.com/file.pdf"));
        assert!(!is_account_path("local/file.pdf"));
        assert!(!is_account_path("file.pdf"));
    }
}
