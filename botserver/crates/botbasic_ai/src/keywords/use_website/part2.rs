use super::*;

/// Register USE_WEBSITE as a regular function instead of custom syntax
/// This avoids conflicts with other USE keywords (USE MODEL, USE KB, etc.)
pub fn register_use_website_function(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    // Register USE_WEBSITE(url, refresh) with both parameters (uppercase)
    engine.register_fn("USE_WEBSITE", move |url: &str, refresh: &str| -> Dynamic {
        trace!(
            "USE_WEBSITE function called: {} REFRESH {} for session: {}",
            url,
            refresh,
            user_clone.id
        );

        let is_valid = url.starts_with("http://") || url.starts_with("https://");
        if !is_valid {
            return Dynamic::from(format!(
                "ERROR: Invalid URL format: {}. Must start with http:// or https://",
                url
            ));
        }

        let state_for_task = Arc::clone(&state_clone);
        let user_for_task = user_clone.clone();
        let url_for_task = url.to_string();
        let refresh_for_task = refresh.to_string();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let _rt = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(_rt) => _rt,
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to build tokio runtime: {}", e)));
                    return;
                }
            };
            let result = associate_website_with_session_refresh(
                &*state_for_task,
                &user_for_task,
                &url_for_task,
                &refresh_for_task,
            );
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(3)) {
            Ok(Ok(message)) => Dynamic::from(message),
            Ok(Err(e)) => Dynamic::from(format!("ERROR: {}", e)),
            Err(_) => Dynamic::from("Website association scheduled."),
        }
    });

    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user.clone();

    // Register use_website(url, refresh) with both parameters (lowercase for preprocessor)
    engine.register_fn("use_website", move |url: &str, refresh: &str| -> Dynamic {
        trace!(
            "use_website function called: {} REFRESH {} for session: {}",
            url,
            refresh,
            user_clone2.id
        );

        let is_valid = url.starts_with("http://") || url.starts_with("https://");
        if !is_valid {
            return Dynamic::from(format!(
                "ERROR: Invalid URL format: {}. Must start with http:// or https://",
                url
            ));
        }

        let state_for_task = Arc::clone(&state_clone2);
        let user_for_task = user_clone2.clone();
        let url_for_task = url.to_string();
        let refresh_for_task = refresh.to_string();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let _rt = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(_rt) => _rt,
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to build tokio runtime: {}", e)));
                    return;
                }
            };
            let result = associate_website_with_session_refresh(
                &*state_for_task,
                &user_for_task,
                &url_for_task,
                &refresh_for_task,
            );
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(3)) {
            Ok(Ok(message)) => Dynamic::from(message),
            Ok(Err(e)) => Dynamic::from(format!("ERROR: {}", e)),
            Err(_) => Dynamic::from("Website association scheduled."),
        }
    });

    let state_clone3 = Arc::clone(&state);
    let user_clone3 = user.clone();

    // Register USE_WEBSITE(url) with just URL (default refresh)
    engine.register_fn("USE_WEBSITE", move |url: &str| -> Dynamic {
        trace!(
            "USE_WEBSITE function called: {} for session: {}",
            url,
            user_clone3.id
        );

        let is_valid = url.starts_with("http://") || url.starts_with("https://");
        if !is_valid {
            return Dynamic::from(format!(
                "ERROR: Invalid URL format: {}. Must start with http:// or https://",
                url
            ));
        }

        let state_for_task = Arc::clone(&state_clone3);
        let user_for_task = user_clone3.clone();
        let url_for_task = url.to_string();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let _rt = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(_rt) => _rt,
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to build tokio runtime: {}", e)));
                    return;
                }
            };
            let result =
                associate_website_with_session(&*state_for_task, &user_for_task, &url_for_task);
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(3)) {
            Ok(Ok(message)) => Dynamic::from(message),
            Ok(Err(e)) => Dynamic::from(format!("ERROR: {}", e)),
            Err(_) => Dynamic::from("Website association scheduled."),
        }
    });

    let state_clone4 = Arc::clone(&state);
    let user_clone4 = user;

    // Register use_website(url) with just URL (default refresh, lowercase)
    engine.register_fn("use_website", move |url: &str| -> Dynamic {
        trace!(
            "use_website function called: {} for session: {}",
            url,
            user_clone4.id
        );

        let is_valid = url.starts_with("http://") || url.starts_with("https://");
        if !is_valid {
            return Dynamic::from(format!(
                "ERROR: Invalid URL format: {}. Must start with http:// or https://",
                url
            ));
        }

        let state_for_task = Arc::clone(&state_clone4);
        let user_for_task = user_clone4.clone();
        let url_for_task = url.to_string();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let _rt = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
            {
                Ok(_rt) => _rt,
                Err(e) => {
                    let _ = tx.send(Err(format!("Failed to build tokio runtime: {}", e)));
                    return;
                }
            };
            let result =
                associate_website_with_session(&*state_for_task, &user_for_task, &url_for_task);
            let _ = tx.send(result);
        });

        match rx.recv_timeout(std::time::Duration::from_secs(3)) {
            Ok(Ok(message)) => Dynamic::from(message),
            Ok(Err(e)) => Dynamic::from(format!("ERROR: {}", e)),
            Err(_) => Dynamic::from("Website association scheduled."),
        }
    });

    info!("Registered USE_WEBSITE and use_website as function (preprocessed from USE WEBSITE)");
}
pub(super) fn associate_website_with_session(
    state: &dyn BasicRuntime,
    user: &UserSession,
    url: &str,
) -> Result<String, String> {
    associate_website_with_session_refresh(state, user, url, "1m") // Default: 1 month
}
pub(super) fn associate_website_with_session_refresh(
    state: &dyn BasicRuntime,
    user: &UserSession,
    url: &str,
    refresh_interval: &str,
) -> Result<String, String> {
    info!(
        "Associating website {} with session {} (refresh: {})",
        url, user.id, refresh_interval
    );

    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    // Get bot name for collection naming
    #[derive(QueryableByName)]
    struct BotName {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }

    let bot_name_result: BotName = diesel::sql_query("SELECT name FROM bots WHERE id = $1")
        .bind::<diesel::sql_types::Uuid, _>(&user.bot_id)
        .get_result(&mut conn)
        .map_err(|e| format!("Failed to get bot name: {}", e))?;

    let bot_id_short = user.bot_id.to_string().chars().take(8).collect::<String>();
    let collection_name = format!(
        "{}_{}_website_{}",
        bot_name_result.name,
        bot_id_short,
        sanitize_url_for_collection(url)
    );

    let website_status = check_website_crawl_status(&mut conn, &user.bot_id, url)?;

    match website_status {
        WebsiteCrawlStatus::NotRegistered => {
            // Auto-register website for crawling instead of failing
            info!(
                "Website {} not registered, auto-registering for crawling with refresh: {}",
                url, refresh_interval
            );
            register_website_for_crawling_with_refresh(
                &mut conn,
                &user.bot_id,
                url,
                refresh_interval,
            )
            .map_err(|e| format!("Failed to register website: {}", e))?;

            // ADD TO SESSION EVEN IF CRAWL IS PENDING!
            // Otherwise kb_context will think the session has no website associated if start.bas only runs once.
            add_website_to_session(&mut conn, &user.id, &user.bot_id, url, &collection_name)?;

            return Ok(format!(
                "Website {} has been registered for crawling (refresh: {}). It will be available once crawling completes.",
                url, refresh_interval
            ));
        }
        WebsiteCrawlStatus::Pending => {
            info!("Website {} is pending crawl, associating anyway", url);
            // Update refresh policy if needed
            update_refresh_policy_if_shorter(&mut conn, &user.bot_id, url, refresh_interval)?;
        }
        WebsiteCrawlStatus::Crawled => {
            info!("Website {} is already crawled and ready", url);
            // Update refresh policy if needed
            update_refresh_policy_if_shorter(&mut conn, &user.bot_id, url, refresh_interval)?;
        }
        WebsiteCrawlStatus::Failed => {
            return Err(format!(
                "Website {} crawling failed. Please check the logs.",
                url
            ));
        }
    }

    add_website_to_session(&mut conn, &user.id, &user.bot_id, url, &collection_name)?;

    Ok(format!(
        "Website {} is now available in this conversation.",
        url
    ))
}
