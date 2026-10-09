use super::*;

pub fn add_member_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["ADD_MEMBER", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let group_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let user_email = context.eval_expression_tree(&inputs[1])?.to_string();
                let role = context.eval_expression_tree(&inputs[2])?.to_string();

                trace!(
                    "ADD_MEMBER: group={}, user_email={}, role={} for user={}",
                    group_id,
                    user_email,
                    role,
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

                    let send_err = if let Ok(_rt) = rt {
                        let result = execute_add_member(
                            &*state_for_task,
                            &user_for_task,
                            &group_id,
                            &user_email,
                            &role,
                        );
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send ADD_MEMBER result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(10)) {
                    Ok(Ok(member_id)) => Ok(Dynamic::from(member_id)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("ADD_MEMBER failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "ADD_MEMBER timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("ADD_MEMBER thread failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user;

    if let Err(e) = engine
        .register_custom_syntax(
            ["CREATE_TEAM", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let name = context.eval_expression_tree(&inputs[0])?.to_string();
                let members_input = context.eval_expression_tree(&inputs[1])?;
                let workspace_template = context.eval_expression_tree(&inputs[2])?.to_string();

                let mut members = Vec::new();
                if members_input.is_array() {
                    let arr = members_input.cast::<rhai::Array>();
                    for item in arr.iter() {
                        members.push(item.to_string());
                    }
                } else {
                    members.push(members_input.to_string());
                }

                trace!(
                    "CREATE_TEAM: name={}, members={:?}, template={} for user={}",
                    name,
                    members,
                    workspace_template,
                    user_clone2.user_id
                );

                let state_for_task = Arc::clone(&state_clone2);
                let user_for_task = user_clone2.clone();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(_rt) = rt {
                        let result = execute_create_team(
                            &*state_for_task,
                            &user_for_task,
                            &name,
                            members,
                            &workspace_template,
                        );
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send CREATE_TEAM result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(15)) {
                    Ok(Ok(team_id)) => Ok(Dynamic::from(team_id)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("CREATE_TEAM failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "CREATE_TEAM timed out".into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
