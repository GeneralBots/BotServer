use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{debug, trace};
use rhai::{Dynamic, Engine};

pub fn post_to_at_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine
        .register_custom_syntax(
            [
                "POST", "TO", "$expr$", "AT", "$expr$", "$expr$", ",", "$expr$",
            ],
            false,
            move |context, inputs| {
                let platform = context.eval_expression_tree(&inputs[0])?.to_string();
                let schedule_time = context.eval_expression_tree(&inputs[1])?.to_string();
                let media = context.eval_expression_tree(&inputs[2])?.to_string();
                let caption = context.eval_expression_tree(&inputs[3])?.to_string();

                let platform = platform.trim_matches('"').to_lowercase();
                let schedule_time = schedule_time.trim_matches('"');
                let media = media.trim_matches('"');
                let caption = caption.trim_matches('"');

                let scheduled_at = parse_schedule_time(schedule_time)?;

                trace!(
                    "POST TO {} AT {}: media={}, caption={}",
                    platform,
                    scheduled_at,
                    media,
                    caption
                );

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let platform_owned = platform;
                let media_owned = media.to_string();
                let caption_owned = caption.to_string();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    if let Ok(_rt) = rt {
                        let result = execute_scheduled_post(
                            &*state_for_task,
                            &user_for_task,
                            &platform_owned,
                            &media_owned,
                            &caption_owned,
                            scheduled_at,
                        );
                        let _ = tx.send(result);
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(Ok(post_id)) => Ok(Dynamic::from(post_id)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("Scheduled POST TO failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "Scheduled POST TO timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    debug!("Registered POST TO AT keyword");
}

fn parse_schedule_time(time_str: &str) -> Result<DateTime<Utc>, Box<rhai::EvalAltResult>> {
    let formats = [
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%d/%m/%Y %H:%M",
        "%m/%d/%Y %H:%M",
    ];

    for format in formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(time_str, format) {
            return Ok(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }

    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
        format!(
            "Invalid date format: {}. Use YYYY-MM-DD HH:MM format.",
            time_str
        )
        .into(),
        rhai::Position::NONE,
    )))
}

fn execute_scheduled_post(
    state: &dyn BasicRuntime,
    user: &UserSession,
    platform: &str,
    media: &str,
    caption: &str,
    scheduled_at: DateTime<Utc>,
) -> Result<String, String> {
    let platforms: Vec<&str> = platform.split(',').map(|s| s.trim()).collect();
    let mut post_ids = Vec::new();

    for p in platforms {
        let post_id = save_scheduled_post(state, user, p, media, caption, scheduled_at)?;
        post_ids.push(post_id);
    }

    Ok(post_ids.join(","))
}

fn save_scheduled_post(
    state: &dyn BasicRuntime,
    user: &UserSession,
    platform: &str,
    media: &str,
    caption: &str,
    scheduled_at: DateTime<Utc>,
) -> Result<String, String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let post_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let user_uuid = Uuid::parse_str(&user.user_id)
        .map_err(|e| format!("Invalid user id: {}", e))?;

    let query = diesel::sql_query(
        "INSERT INTO social_posts (id, bot_id, user_id, platform, content, media_url, status, scheduled_at, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, 'scheduled', $7, $8)",
    )
    .bind::<diesel::sql_types::Text, _>(&post_id)
    .bind::<diesel::sql_types::Uuid, _>(user.bot_id)
    .bind::<diesel::sql_types::Uuid, _>(user_uuid)
    .bind::<diesel::sql_types::Text, _>(platform)
    .bind::<diesel::sql_types::Text, _>(caption)
    .bind::<diesel::sql_types::Text, _>(media)
    .bind::<diesel::sql_types::Timestamptz, _>(&scheduled_at)
    .bind::<diesel::sql_types::Timestamptz, _>(&now);

    query.execute(&mut *conn).map_err(|e| {
        log::error!("Failed to save scheduled post: {}", e);
        format!("Failed to save scheduled post: {}", e)
    })?;

    trace!(
        "Scheduled post saved: {} to {} at {}",
        post_id,
        platform,
        scheduled_at
    );
    Ok(post_id)
}

use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;