use super::*;

pub fn book_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine.register_custom_syntax(
            [
                "BOOK", "$expr$", ",", "$expr$", ",", "$expr$", ",", "$expr$", ",", "$expr$",
            ],
            false,
            move |context, inputs| {
                let title = context.eval_expression_tree(&inputs[0])?.to_string();
                let description = context.eval_expression_tree(&inputs[1])?.to_string();
                let start_time_str = context.eval_expression_tree(&inputs[2])?.to_string();
                let duration_minutes = context
                    .eval_expression_tree(&inputs[3])?
                    .as_int()
                    .unwrap_or(30);
                let location = context.eval_expression_tree(&inputs[4])?.to_string();

                trace!(
                    "BOOK: title={}, start={}, duration={} min for user={}",
                    title,
                    start_time_str,
                    duration_minutes,
                    user_clone.user_id
                );

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_book(
                                &*state_for_task,
                                &user_for_task,
                                &title,
                                &description,
                                &start_time_str,
                                duration_minutes,
                                &location,
                            )
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send BOOK result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                    Ok(Ok(event_id)) => Ok(Dynamic::from(event_id)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("BOOK failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "BOOK timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register a booking keyword: {e}");
    }

    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user.clone();

    if let Err(e) = engine.register_custom_syntax(
            ["BOOK_MEETING", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let meeting_details = context.eval_expression_tree(&inputs[0])?;
                let attendees_input = context.eval_expression_tree(&inputs[1])?;

                let mut attendees = Vec::new();
                if attendees_input.is_array() {
                    let arr = attendees_input.cast::<rhai::Array>();
                    for item in arr.iter() {
                        attendees.push(item.to_string());
                    }
                }

                trace!(
                    "BOOK_MEETING with {} attendees for user={}",
                    attendees.len(),
                    user_clone2.user_id
                );

                let state_for_task = Arc::clone(&state_clone2);
                let user_for_task = user_clone2.clone();
                let meeting_json = meeting_details.to_string();

                let (tx, rx) = std::sync::mpsc::channel();
                let meeting_json_clone = meeting_json.clone();
                let attendees_clone = attendees.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result = match rt {
                        Ok(rt) => rt.block_on(async move {
                            execute_book_meeting(
                                &*state_for_task,
                                &user_for_task,
                                meeting_json_clone,
                                attendees_clone,
                            )
                        }),
                        Err(_) => Err("Failed to create runtime".into()),
                    };
                    let _ = tx.send(result);
                });
                let result = rx.recv().unwrap_or(Err("Failed to create runtime".into()));

                match result {
                    Ok(event_id) => Ok(Dynamic::from(event_id)),
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("BOOK_MEETING failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register a booking keyword: {e}");
    }

    let state_clone3 = Arc::clone(&state);

    if let Err(e) = engine.register_custom_syntax(
            ["CHECK_AVAILABILITY", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let date_str = context.eval_expression_tree(&inputs[0])?.to_string();
                let duration_minutes = context
                    .eval_expression_tree(&inputs[1])?
                    .as_int()
                    .unwrap_or(30);

                trace!(
                    "CHECK_AVAILABILITY for {} on {} for user={}",
                    duration_minutes,
                    date_str,
                    user.user_id
                );

                let state_for_task = Arc::clone(&state_clone3);

                let (tx, rx) = std::sync::mpsc::channel();
                let date_str_clone = date_str.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let result = match rt {
                        Ok(rt) => rt.block_on(async move {
                            check_availability(
                                &*state_for_task,
                                &date_str_clone,
                                duration_minutes,
                            )
                        }),
                        Err(_) => Err("Failed to create runtime".into()),
                    };
                    let _ = tx.send(result);
                });
                let result = rx.recv().unwrap_or(Err("Failed to create runtime".into()));

                match result {
                    Ok(slots) => Ok(Dynamic::from(slots)),
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("CHECK_AVAILABILITY failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register a booking keyword: {e}");
    }
}
pub(super) fn execute_book(
    state: &dyn BasicRuntime,
    user: &UserSession,
    title: &str,
    description: &str,
    start_time_str: &str,
    duration_minutes: i64,
    location: &str,
) -> Result<String, String> {
    let start_time = parse_time_string(start_time_str)?;
    let end_time = start_time + Duration::minutes(duration_minutes);

    let calendar_engine = get_calendar_engine(state)?;

    let conflicts = calendar_engine
        .check_conflicts(start_time, end_time, &user.user_id.to_string())
        .map_err(|e| format!("Failed to check conflicts: {}", e))?;

    if !conflicts.is_empty() {
        return Err(format!(
            "Time slot conflicts with existing appointment: {}",
            conflicts[0].title
        ));
    }

    let event = CalendarEvent {
        id: Uuid::new_v4(),
        title: title.to_string(),
        description: Some(description.to_string()),
        start_time,
        end_time,
        location: if location.is_empty() {
            None
        } else {
            Some(location.to_string())
        },
        organizer: user.user_id.to_string(),
        attendees: vec![user.user_id.to_string()],
        reminder_minutes: Some(15),
        recurrence_rule: None,
        status: EventStatus::Confirmed,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let created_event = calendar_engine
        .create_event(event)
        .map_err(|e| format!("Failed to create appointment: {}", e))?;

    log_booking(state, user, &created_event.id.to_string(), title)?;

    info!(
        "Appointment booked: {} at {} for user {}",
        title, start_time, user.user_id
    );

    Ok(format!(
        "Appointment '{}' booked for {} (ID: {})",
        title,
        start_time.format("%Y-%m-%d %H:%M"),
        created_event.id
    ))
}
