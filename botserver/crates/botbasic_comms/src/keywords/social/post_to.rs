use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use chrono::Utc;
use diesel::prelude::*;
use log::trace;
use rhai::{Dynamic, Engine};

pub fn post_to_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["POST", "TO", "$expr$", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let platform = context.eval_expression_tree(&inputs[0])?.to_string();
                let media = context.eval_expression_tree(&inputs[1])?.to_string();
                let caption = context.eval_expression_tree(&inputs[2])?.to_string();

                let platform = platform.trim_matches('"').to_lowercase();
                let media = media.trim_matches('"');
                let caption = caption.trim_matches('"');

                trace!("POST TO {}: media={}, caption={}", platform, media, caption);

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
                        let result = execute_post_to(
                            &*state_for_task,
                            &user_for_task,
                            &platform_owned,
                            &media_owned,
                            &caption_owned,
                        );
                        let _ = tx.send(result);
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(post_id)) => Ok(Dynamic::from(post_id)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("POST TO failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "POST TO timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    register_platform_shortcuts(state, user, engine);
}

fn register_platform_shortcuts(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    for platform in &["INSTAGRAM", "FACEBOOK", "LINKEDIN", "TWITTER"] {
        let state_clone = Arc::clone(&state);
        let user_clone = user.clone();
        let platform_lower = platform.to_lowercase();

        if let Err(e) = engine
            .register_custom_syntax(
                ["POST", "TO", platform, "$expr$", ",", "$expr$"],
                false,
                move |context, inputs| {
                    let media = context.eval_expression_tree(&inputs[0])?.to_string();
                    let caption = context.eval_expression_tree(&inputs[1])?.to_string();

                    let media = media.trim_matches('"');
                    let caption = caption.trim_matches('"');

                    let state_for_task = Arc::clone(&state_clone);
                    let user_for_task = user_clone.clone();
                    let platform_owned = platform_lower.clone();
                    let media_owned = media.to_string();
                    let caption_owned = caption.to_string();

                    let (tx, rx) = std::sync::mpsc::channel();

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Builder::new_multi_thread()
                            .worker_threads(2)
                            .enable_all()
                            .build();

                        if let Ok(_rt) = rt {
                            let result = execute_post_to(
                                &*state_for_task,
                                &user_for_task,
                                &platform_owned,
                                &media_owned,
                                &caption_owned,
                            );
                            let _ = tx.send(result);
                        }
                    });

                    match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                        Ok(Ok(post_id)) => Ok(Dynamic::from(post_id)),
                        Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            format!("POST TO failed: {}", e).into(),
                            rhai::Position::NONE,
                        ))),
                        Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "POST TO timed out".into(),
                            rhai::Position::NONE,
                        ))),
                    }
                },
            )
        {
            log::error!("Failed to register the custom syntax: {e}");
        }
    }
}

fn execute_post_to(
    state: &dyn BasicRuntime,
    user: &UserSession,
    platform_input: &str,
    media: &str,
    caption: &str,
) -> Result<String, String> {
    let platforms: Vec<&str> = platform_input.split(',').map(|s| s.trim()).collect();
    let mut post_ids = Vec::new();

    for platform in platforms {
        let post_id = save_social_post(state, user, platform, media, caption)?;
        post_ids.push(post_id);
    }

    Ok(post_ids.join(","))
}

fn save_social_post(
    state: &dyn BasicRuntime,
    user: &UserSession,
    platform: &str,
    media: &str,
    caption: &str,
) -> Result<String, String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let post_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let user_uuid = Uuid::parse_str(&user.user_id)
        .map_err(|e| format!("Invalid user id: {}", e))?;

    let query = diesel::sql_query(
        "INSERT INTO social_posts (id, bot_id, user_id, platform, content, media_url, status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7)",
    )
    .bind::<diesel::sql_types::Text, _>(&post_id)
    .bind::<diesel::sql_types::Uuid, _>(user.bot_id)
    .bind::<diesel::sql_types::Uuid, _>(user_uuid)
    .bind::<diesel::sql_types::Text, _>(platform)
    .bind::<diesel::sql_types::Text, _>(caption)
    .bind::<diesel::sql_types::Text, _>(media)
    .bind::<diesel::sql_types::Timestamptz, _>(&now);

    query.execute(&mut *conn).map_err(|e| {
        log::error!("Failed to save social post: {}", e);
        format!("Failed to save post: {}", e)
    })?;

    trace!("Social post saved: {} to {}", post_id, platform);
    Ok(post_id)
}

use std::sync::Arc;
use uuid::Uuid;