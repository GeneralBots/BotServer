use super::*;

/// Parse refresh interval string (e.g., "1d", "1w", "1m", "1y") into days
/// Returns the number of days for the refresh interval
pub(super) fn parse_refresh_interval(interval: &str) -> Result<i32, String> {
    let interval_lower = interval.trim().to_lowercase();

    // Match patterns like "1d", "7d", "2w", "1m", "1y", etc.
    if interval_lower.ends_with('d') {
        let days: i32 = interval_lower[..interval_lower.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid days format: {}", interval))?;
        Ok(days)
    } else if interval_lower.ends_with('w') {
        let weeks: i32 = interval_lower[..interval_lower.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid weeks format: {}", interval))?;
        Ok(weeks * 7)
    } else if interval_lower.ends_with('m') {
        let months: i32 = interval_lower[..interval_lower.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid months format: {}", interval))?;
        Ok(months * 30) // Approximate month as 30 days
    } else if interval_lower.ends_with('y') {
        let years: i32 = interval_lower[..interval_lower.len() - 1]
            .parse()
            .map_err(|_| format!("Invalid years format: {}", interval))?;
        Ok(years * 365) // Approximate year as 365 days
    } else {
        // Try to parse as plain number (assume days)
        interval.parse().map_err(|_| {
            format!(
                "Invalid refresh interval format: {}. Use format like '1d', '1w', '1m', '1y'",
                interval
            )
        })
    }
}
/// Convert days to expires_policy string format
pub(super) fn days_to_expires_policy(days: i32) -> String {
    format!("{}d", days)
}
pub fn use_website_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    // Register syntax for USE WEBSITE "url" REFRESH "interval" (case insensitive)
    // Register both uppercase and lowercase variants
if let Err(e) =     engine
        .register_custom_syntax(
            ["USE", "WEBSITE", "$expr$", "REFRESH", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?;
                let url_str = url.to_string().trim_matches('"').to_string();

                let refresh = context.eval_expression_tree(&inputs[1])?;
                let refresh_str = refresh.to_string().trim_matches('"').to_string();

                trace!(
                    "USE WEBSITE command executed: {} REFRESH {} for session: {}",
                    url_str,
                    refresh_str,
                    user_clone.id
                );

                let is_valid = url_str.starts_with("http://") || url_str.starts_with("https://");
                if !is_valid {
                    return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "Invalid URL format. Must start with http:// or https://".into(),
                        rhai::Position::NONE,
                    )));
                }

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let url_for_task = url_str;
                let refresh_for_task = refresh_str;
                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(_rt) = rt {
                        let result = associate_website_with_session_refresh(
                            &*state_for_task,
                            &user_for_task,
                            &url_for_task,
                            &refresh_for_task,
                        );
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(3)) {
                    Ok(Ok(message)) => Ok(Dynamic::from(message)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        e.into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Ok(Dynamic::from("Website association scheduled.")),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    // Register syntax for USE WEBSITE "url" (without REFRESH)
    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["USE", "WEBSITE", "$expr$"],
            false,
            move |context, inputs| {
                let url = context.eval_expression_tree(&inputs[0])?;
                let url_str = url.to_string().trim_matches('"').to_string();

                trace!(
                    "USE WEBSITE command executed: {} for session: {}",
                    url_str,
                    user_clone2.id
                );

                let is_valid = url_str.starts_with("http://") || url_str.starts_with("https://");
                if !is_valid {
                    return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "Invalid URL format. Must start with http:// or https://".into(),
                        rhai::Position::NONE,
                    )));
                }

                let state_for_task = Arc::clone(&state_clone2);
                let user_for_task = user_clone2.clone();
                let url_for_task = url_str;
                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(_rt) = rt {
                        let result = associate_website_with_session(
                            &*state_for_task,
                            &user_for_task,
                            &url_for_task,
                        );
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(3)) {
                    Ok(Ok(message)) => Ok(Dynamic::from(message)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        e.into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Ok(Dynamic::from("Website association scheduled.")),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
