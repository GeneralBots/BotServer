use super::*;

pub fn register_transfer_to_human_keyword(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    let state_clone = state.clone();
    let user_clone = user.clone();
    engine.register_fn("transfer_to_human", move || -> Dynamic {
        let state = state_clone.clone();
        let session = user_clone.clone();

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(async {
                    execute_transfer(
                        state,
                        &session,
                        TransferToHumanRequest {
                            name: None,
                            department: None,
                            priority: None,
                            reason: None,
                            context: None,
                        },
                    )
                    .await
                }),
                Err(_) => TransferResult {
                    success: false,
                    status: TransferStatus::Error,
                    queue_position: None,
                    assigned_to: None,
                    assigned_to_name: None,
                    estimated_wait_seconds: None,
                    message: "Failed to create runtime".to_string(),
                },
            };
            let _ = tx.send(result);
        });
        let result = rx.recv().unwrap_or_else(|_| TransferResult {
            success: false,
            status: TransferStatus::Error,
            queue_position: None,
            assigned_to: None,
            assigned_to_name: None,
            estimated_wait_seconds: None,
            message: "Channel error".to_string(),
        });
        result.to_dynamic()
    });

    let state_clone = state.clone();
    let user_clone = user.clone();
    engine.register_fn("transfer_to_human", move |name: &str| -> Dynamic {
        let state = state_clone.clone();
        let session = user_clone.clone();
        let name_str = name.to_string();

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(async {
                    execute_transfer(
                        state,
                        &session,
                        TransferToHumanRequest {
                            name: Some(name_str),
                            department: None,
                            priority: None,
                            reason: None,
                            context: None,
                        },
                    )
                    .await
                }),
                Err(_) => TransferResult {
                    success: false,
                    status: TransferStatus::Error,
                    queue_position: None,
                    assigned_to: None,
                    assigned_to_name: None,
                    estimated_wait_seconds: None,
                    message: "Failed to create runtime".to_string(),
                },
            };
            let _ = tx.send(result);
        });
        let result = rx.recv().unwrap_or_else(|_| TransferResult {
            success: false,
            status: TransferStatus::Error,
            queue_position: None,
            assigned_to: None,
            assigned_to_name: None,
            estimated_wait_seconds: None,
            message: "Channel error".to_string(),
        });
        result.to_dynamic()
    });

    let state_clone = state.clone();
    let user_clone = user.clone();
    engine.register_fn(
        "transfer_to_human",
        move |department: &str, priority: &str| -> Dynamic {
            let state = state_clone.clone();
            let session = user_clone.clone();
            let dept = department.to_string();
            let prio = priority.to_string();

            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let result = match rt {
                    Ok(rt) => rt.block_on(async {
                        execute_transfer(
                            state,
                            &session,
                            TransferToHumanRequest {
                                name: None,
                                department: Some(dept),
                                priority: Some(prio),
                                reason: None,
                                context: None,
                            },
                        )
                        .await
                    }),
                    Err(_) => TransferResult {
                        success: false,
                        status: TransferStatus::Error,
                        queue_position: None,
                        assigned_to: None,
                        assigned_to_name: None,
                        estimated_wait_seconds: None,
                        message: "Failed to create runtime".to_string(),
                    },
                };
                let _ = tx.send(result);
            });
            let result = rx.recv().unwrap_or_else(|_| TransferResult {
                success: false,
                status: TransferStatus::Error,
                queue_position: None,
                assigned_to: None,
                assigned_to_name: None,
                estimated_wait_seconds: None,
                message: "Channel error".to_string(),
            });
            result.to_dynamic()
        },
    );

    let state_clone = state.clone();
    let user_clone = user.clone();
    engine.register_fn(
        "transfer_to_human",
        move |department: &str, priority: &str, reason: &str| -> Dynamic {
            let state = state_clone.clone();
            let session = user_clone.clone();
            let dept = department.to_string();
            let prio = priority.to_string();
            let rsn = reason.to_string();

            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let result = match rt {
                    Ok(rt) => rt.block_on(async {
                        execute_transfer(
                            state,
                            &session,
                            TransferToHumanRequest {
                                name: None,
                                department: Some(dept),
                                priority: Some(prio),
                                reason: Some(rsn),
                                context: None,
                            },
                        )
                        .await
                    }),
                    Err(_) => TransferResult {
                        success: false,
                        status: TransferStatus::Error,
                        queue_position: None,
                        assigned_to: None,
                        assigned_to_name: None,
                        estimated_wait_seconds: None,
                        message: "Failed to create runtime".to_string(),
                    },
                };
                let _ = tx.send(result);
            });
            let result = rx.recv().unwrap_or_else(|_| TransferResult {
                success: false,
                status: TransferStatus::Error,
                queue_position: None,
                assigned_to: None,
                assigned_to_name: None,
                estimated_wait_seconds: None,
                message: "Channel error".to_string(),
            });
            result.to_dynamic()
        },
    );

    let state_clone = state;
    let user_clone = user;
    engine.register_fn("transfer_to_human_ex", move |params: Map| -> Dynamic {
        let state = state_clone.clone();
        let session = user_clone.clone();

        let name = params
            .get("name")
            .and_then(|v| v.clone().try_cast::<String>());
        let department = params
            .get("department")
            .and_then(|v| v.clone().try_cast::<String>());
        let priority = params
            .get("priority")
            .and_then(|v| v.clone().try_cast::<String>());
        let reason = params
            .get("reason")
            .and_then(|v| v.clone().try_cast::<String>());
        let context = params
            .get("context")
            .and_then(|v| v.clone().try_cast::<String>());

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let result = match rt {
                Ok(rt) => rt.block_on(async {
                    execute_transfer(
                        state,
                        &session,
                        TransferToHumanRequest {
                            name,
                            department,
                            priority,
                            reason,
                            context,
                        },
                    )
                    .await
                }),
                Err(_) => TransferResult {
                    success: false,
                    status: TransferStatus::Error,
                    queue_position: None,
                    assigned_to: None,
                    assigned_to_name: None,
                    estimated_wait_seconds: None,
                    message: "Failed to create runtime".to_string(),
                },
            };
            let _ = tx.send(result);
        });
        let result = rx.recv().unwrap_or_else(|_| TransferResult {
            success: false,
            status: TransferStatus::Error,
            queue_position: None,
            assigned_to: None,
            assigned_to_name: None,
            estimated_wait_seconds: None,
            message: "Channel error".to_string(),
        });
        result.to_dynamic()
    });

    debug!("Registered TRANSFER TO HUMAN keywords");
}
