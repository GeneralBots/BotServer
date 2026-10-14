pub use botwhatsapp::*;

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use botcore::shared::state::AppState;
use botcoresecrets::manager::SecretsManager;
use uuid::Uuid;

use diesel::RunQueryDsl;
use crate::core::bot::pipeline::{self, ChannelSink, PipelineError, PipelineResult};

#[derive(diesel::QueryableByName)]
struct BotLookupRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}

fn find_bot_by_phone_number_id(pool: &DbPool, phone_number_id: &str) -> Option<(Uuid, String)> {
    let mut conn = pool.get_timeout(std::time::Duration::from_secs(2)).ok()?;
    diesel::sql_query(
        "SELECT b.id, b.name FROM bots b \
         JOIN bot_configuration bc ON bc.bot_id = b.id \
         WHERE bc.config_key = 'whatsapp-phone-number-id' \
         AND bc.config_value = $1 \
         AND b.is_active = true \
         LIMIT 1"
    )
    .bind::<diesel::sql_types::Text, _>(phone_number_id)
    .get_result::<BotLookupRow>(&mut conn)
    .ok()
    .map(|r| (r.id, r.name))
}

fn find_first_active_bot(pool: &DbPool) -> Option<(Uuid, String)> {
    let mut conn = pool.get_timeout(std::time::Duration::from_secs(2)).ok()?;
    // Prefer cristo bot if it exists and is active
    if let Ok(row) = diesel::sql_query(
        "SELECT id, name FROM bots WHERE name = 'cristo' AND is_active = true LIMIT 1"
    ).get_result::<BotLookupRow>(&mut conn) {
        return Some((row.id, row.name));
    }
    // Fallback to first active bot
    diesel::sql_query(
        "SELECT id, name FROM bots WHERE is_active = true ORDER BY created_at ASC LIMIT 1"
    )
    .get_result::<BotLookupRow>(&mut conn)
    .ok()
    .map(|r| (r.id, r.name))
}

fn strip_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    let decoded = result.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
        .replace("&quot;", "\"").replace("&#39;", "'").replace("&nbsp;", " ");
    let cleaned = decoded.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    cleaned
}

struct WABufferedSink {
    send_msg: botwhatsapp::state::SendMessageFn,
    bot_name: String,
    phone: String,
    buffer: Arc<tokio::sync::Mutex<String>>,
}

#[async_trait::async_trait]
impl ChannelSink for WABufferedSink {
    async fn send_bot_response(&self, response: &botlib::models::BotResponse) -> PipelineResult<()> {
        let mut buf = self.buffer.lock().await;
        let clean = strip_html(&response.content);
        buf.push_str(&clean);
        if response.is_complete {
            let full = std::mem::take(&mut *buf);
            if !full.is_empty() {
                (self.send_msg)(&self.phone, &full, &self.bot_name).await
                    .map_err(PipelineError::Transport)?;
            }
        }
        Ok(())
    }

    async fn send_error(&self, _session_id: &str, message: &str) -> PipelineResult<()> {
        let mut buf = self.buffer.lock().await;
        let clean = strip_html(message);
        buf.push_str(&clean);
        let full = std::mem::take(&mut *buf);
        if !full.is_empty() {
            (self.send_msg)(&self.phone, &full, &self.bot_name).await
                .map_err(PipelineError::Transport)?;
        }
        Ok(())
    }

    fn channel_type(&self) -> &str { "whatsapp" }
    fn supports_streaming(&self) -> bool { false }
    fn supports_suggestions(&self) -> bool { false }
}

fn load_wa_config(pool: &DbPool, bot_id: &Uuid, key: &str) -> Option<String> {
    use botcore::config::ConfigManager;
    let cfg = ConfigManager::new(pool.clone());
    cfg.get_config(bot_id, key, None).ok().filter(|v| !v.is_empty())
}

fn load_wa_config_from_vault(bot_id: &Uuid) -> Option<HashMap<String, String>> {
    if let Ok(sm) = SecretsManager::get() {
        if sm.is_enabled() {
            let vault_path = format!("gbo/00000000-0000-0000-0000-000000000000/{}/whatsapp", bot_id);
            let vault_path_for_log = vault_path.clone();
            let self_owned = sm.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
                let result = if let Ok(rt) = rt {
                    rt.block_on(async move {
                        self_owned.get_secret(&vault_path).await.ok()
                    })
                } else { None };
                let _ = tx.send(result);
            });
            if let Ok(Some(secrets)) = rx.recv_timeout(std::time::Duration::from_secs(5)) {
                log::info!("WhatsApp config loaded from Vault path: {}", vault_path_for_log);
                return Some(secrets);
            }
        }
    }
    None
}

pub fn configure(app_state: &Arc<AppState>) -> Router<()> {
    let pool = app_state.conn.clone();
    let default_bot = find_first_active_bot(&pool)
        .unwrap_or_else(|| (Uuid::nil(), "default".to_string()));
    log::info!("WhatsApp module: default bot_id={} name='{}'", default_bot.0, default_bot.1);

    let vault_wa = load_wa_config_from_vault(&default_bot.0);

    let cfg_phone_number_id = vault_wa.as_ref()
        .and_then(|m| m.get("whatsapp_phone_number_id").cloned())
        .or_else(|| load_wa_config(&pool, &default_bot.0, "whatsapp-phone-number-id"))
        .unwrap_or_default();
    let cfg_api_key = vault_wa.as_ref()
        .and_then(|m| m.get("whatsapp_api_key").cloned())
        .or_else(|| load_wa_config(&pool, &default_bot.0, "whatsapp-api-key"))
        .unwrap_or_default();
    let cfg_verify_token = vault_wa.as_ref()
        .and_then(|m| m.get("whatsapp_verify_token").cloned())
        .or_else(|| load_wa_config(&pool, &default_bot.0, "whatsapp-verify-token"))
        .unwrap_or_default();
    let cfg_business_account_id = vault_wa.as_ref()
        .and_then(|m| m.get("whatsapp_business_account_id").cloned())
        .or_else(|| load_wa_config(&pool, &default_bot.0, "whatsapp-business-account-id"))
        .unwrap_or_default();
    let cfg_api_url = vault_wa.as_ref()
        .and_then(|m| m.get("whatsapp_api_url").cloned())
        .or_else(|| load_wa_config(&pool, &default_bot.0, "whatsapp-api-url"))
        .unwrap_or_else(|| "https://graph.facebook.com/v21.0".to_string());

    let sm_pni = cfg_phone_number_id.clone();
    let sm_key = cfg_api_key.clone();
    let sm_url = cfg_api_url.clone();

    let send_message_fn: botwhatsapp::state::SendMessageFn = Arc::new(move |to: &str, message: &str, _bot_name: &str| {
        let pni = sm_pni.clone();
        let key = sm_key.clone();
        let burl = sm_url.clone();
        let to = to.to_string();
        let message = message.to_string();
        Box::pin(async move {
            if pni.is_empty() || key.is_empty() {
                log::warn!("WhatsApp not configured: missing phone_number_id or api_key");
                return Err("WhatsApp not configured".to_string());
            }
            let url = format!("{}/{}/messages", burl, pni);
            let body = serde_json::json!({
                "messaging_product": "whatsapp",
                "to": to,
                "type": "text",
                "text": { "body": message }
            });
            let cl = reqwest::Client::builder()
                .user_agent("GeneralBots/1.0")
                .build().unwrap_or_default();
            let resp = cl
                .post(&url)
                .header("Authorization", format!("Bearer {}", key))
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("HTTP send error: {}", e))?;
            let status = resp.status();
            if status.is_success() {
                log::info!("WhatsApp message sent to {}", to);
                Ok(())
            } else {
                let body_text = resp.text().await.unwrap_or_default();
                let err_msg = format!("WhatsApp API error {}: {}", status, body_text);
                log::error!("{}", err_msg);
                Err(err_msg)
            }
        })
    });

    let sm_for_process = send_message_fn.clone();

    let gc_pni = cfg_phone_number_id.clone();
    let gc_ba = cfg_business_account_id.clone();
    let gc_url = cfg_api_url.clone();

    let sc_vt = cfg_verify_token.clone();
    let sc_ak = cfg_api_key.clone();

    let wa_state = Arc::new(WhatsAppState {
        pool: pool.clone(),
        send_message: send_message_fn,
        get_default_bot: {
            let pool_for_default = pool.clone();
            Arc::new(move |_c: &mut diesel::PgConnection| {
                find_first_active_bot(&pool_for_default)
                    .unwrap_or_else(|| (Uuid::nil(), "default".to_string()))
            })
        },
        find_bot: {
            let pool_for_find = pool.clone();
            Arc::new(move |phone_number_id: &str| {
                find_bot_by_phone_number_id(&pool_for_find, phone_number_id)
                    .or_else(|| find_first_active_bot(&pool_for_find))
                    .unwrap_or_else(|| (Uuid::nil(), "default".to_string()))
            })
        },
        get_config: {
            Arc::new(move |key: &str| -> Result<String, String> {
                match key {
                    "whatsapp_api_url" => Ok(gc_url.clone()),
                    "whatsapp_phone_number_id" => Ok(gc_pni.clone()),
                    "whatsapp_business_account_id" => Ok(gc_ba.clone()),
                    _ => Err(format!("Config key '{}' not found", key)),
                }
            })
        },
        secrets: {
            Arc::new(move |key: &str| -> Result<String, String> {
                match key {
                    "whatsapp_verify_token" => Ok(sc_vt.clone()),
                    "whatsapp_api_key" => Ok(sc_ak.clone()),
                    _ => Err(format!("Secret key '{}' not found", key)),
                }
            })
        },
        put_media: crate::main_module::routes::channel_support::make_wa_put_media_fn(app_state),
        transcribe_audio: Arc::new(|_data: &[u8]| {
            Box::pin(async move { Err("Audio transcription not available".to_string()) })
        }),
        process_message: {
            let app_state = app_state.clone();
            let sm_fn = sm_for_process.clone();
            Arc::new(move |bot_id_str: String, phone: String, content: String, session_id_str: String, bot_name: String| {
                let state = app_state.clone();
                let sm = sm_fn.clone();
                Box::pin(async move {
                    log::info!("WhatsApp process msg from {} for bot '{}' (name={}): {}", phone, bot_id_str, bot_name, content);

                    let session_id = Uuid::parse_str(&session_id_str).unwrap_or_else(|_| Uuid::new_v4());
                    let user_id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("wa:{}", phone).as_bytes());

                    let msg = botlib::models::UserMessage::text(
                        bot_name, user_id.to_string(), session_id.to_string(),
                        "whatsapp", &content,
                    );

                    let sink = WABufferedSink {
                        send_msg: sm,
                        bot_name: bot_id_str.clone(),
                        phone: phone.clone(),
                        buffer: Arc::new(tokio::sync::Mutex::new(String::new())),
                    };

                    let _ = pipeline::run_pipeline_for_channel(
                        &state, &msg, &sink,
                    ).await;

                    Ok(())
                })
            })
        },
        user_lookup: Arc::new(|_identifier: &str| {
            Box::pin(async move { Ok(None::<String>) })
        }),
        user_create: Arc::new(
            |_identifier: &str, _display_name: &str, _email: &str, _phone: Option<&str>| {
                Box::pin(async move { Ok("00000000-0000-0000-0000-000000000000".to_string()) })
            },
        ),
    });

    botwhatsapp::configure_whatsapp_routes().with_state(wa_state)
}