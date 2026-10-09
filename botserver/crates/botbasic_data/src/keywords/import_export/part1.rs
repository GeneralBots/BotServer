use super::*;

pub fn register_import_export(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_import_keyword(Arc::clone(&state), user.clone(), engine);
    register_export_keyword(state, user, engine);
}
pub fn register_import_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    if let Err(e) = engine
        .register_custom_syntax(["IMPORT", "$expr$"], false, move |context, inputs| {
            let file_path = context.eval_expression_tree(&inputs[0])?.to_string();

            trace!("IMPORT: Loading data from {}", file_path);

            let state_for_task = Arc::clone(&state_clone);
            let user_for_task = user.clone();

            let (tx, rx) = std::sync::mpsc::channel::<Result<Value, String>>();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(_rt) = rt {
                    let result = execute_import_json(state_for_task, &user_for_task, &file_path);
                    tx.send(result).err()
                } else {
                    tx.send(Err("Failed to build tokio runtime".into())).err()
                };

                if send_err.is_some() {
                    log::error!("Failed to send IMPORT result from thread");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                Ok(Ok(json_result)) => Ok(json_to_dynamic(&json_result)),
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("IMPORT failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "IMPORT timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("IMPORT thread failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
            }
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_export_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);

    if let Err(e) = engine
        .register_custom_syntax(
            ["EXPORT", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let file_path = context.eval_expression_tree(&inputs[0])?.to_string();
                let data = context.eval_expression_tree(&inputs[1])?;

                trace!("EXPORT: Saving data to {}", file_path);

                let data_json = dynamic_to_json_value(&data);

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user.clone();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(_rt) = rt {
let result = execute_export_json(
            state_for_task,
            &user_for_task,
                            &file_path,
                            data_json,
                        );
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send EXPORT result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(result)) => Ok(Dynamic::from(result)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("EXPORT failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "EXPORT timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("EXPORT thread failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub(super) fn execute_import_json(
    state: Arc<dyn BasicRuntime>,
    user: &UserSession,
    file_path: &str,
) -> Result<Value, String> {
    match execute_import(state, user, file_path) {
        Ok(dynamic) => Ok(dynamic_to_json(&dynamic)),
        Err(e) => Err(e.to_string()),
    }
}
pub(super) fn execute_export_json(
    state: Arc<dyn BasicRuntime>,
    user: &UserSession,
    file_path: &str,
    data_json: Value,
) -> Result<String, String> {
    let data = json_to_dynamic(&data_json);
    match execute_export(state, user, file_path, data) {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}
pub(super) fn dynamic_to_json_value(data: &Dynamic) -> Value {
    dynamic_to_json(data)
}
pub(super) fn execute_import(
    state: Arc<dyn BasicRuntime>,
    user: &UserSession,
    file_path: &str,
) -> Result<Dynamic, Box<dyn std::error::Error + Send + Sync>> {
    let full_path = resolve_file_path(state, user, file_path)?;
    let extension = Path::new(&full_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "csv" => import_csv(&full_path),
        "json" => import_json(&full_path),
        "xlsx" | "xls" => {
            #[cfg(feature = "sheet")]
            {
                import_excel(&full_path)
            }
            #[cfg(not(feature = "sheet"))]
            {
                Err(format!("Excel import requires 'sheet' feature. File: {}", file_path).into())
            }
        }
        "tsv" => import_tsv(&full_path),
        _ => Err(format!("Unsupported file format: .{}", extension).into()),
    }
}
pub(super) fn execute_export(
    state: Arc<dyn BasicRuntime>,
    user: &UserSession,
    file_path: &str,
    data: Dynamic,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let full_path = resolve_file_path(state, user, file_path)?;
    let extension = Path::new(&full_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "csv" => export_csv(&full_path, data),
        "json" => export_json(&full_path, data),
        "xlsx" => {
            #[cfg(feature = "sheet")]
            {
                export_excel(&full_path, data)
            }
            #[cfg(not(feature = "sheet"))]
            {
                Err(format!("Excel export requires 'sheet' feature. File: {}", file_path).into())
            }
        }
        "tsv" => export_tsv(&full_path, data),
        _ => Err(format!("Unsupported export format: .{}", extension).into()),
    }
}
pub(super) fn resolve_file_path(
    state: Arc<dyn BasicRuntime>,
    user: &UserSession,
    file_path: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    if file_path.starts_with("http://") || file_path.starts_with("https://") {
        return Ok(file_path.to_string());
    }

    if Path::new(file_path).is_absolute() {
        return Ok(file_path.to_string());
    }

    let work = get_work_path();
    let data_dir = state.config_value("data-dir")
        .unwrap_or_else(|| work.clone());
    let base_path = format!("{}/bots/{}/gbdrive", data_dir, user.bot_id);

    std::fs::create_dir_all(&base_path)?;

    Ok(format!("{}/{}", base_path, file_path))
}
pub(super) fn import_csv(file_path: &str) -> Result<Dynamic, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let headers: Vec<String> = match lines.next() {
        Some(Ok(line)) => parse_csv_line(&line),
        _ => return Err("CSV file is empty or has no header".into()),
    };

    let mut results: Array = Array::new();

    for line in lines.map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }

        let values = parse_csv_line(&line);
        let mut row_map: Map = Map::new();

        for (i, header) in headers.iter().enumerate() {
            let value = values.get(i).map(|s| s.as_str()).unwrap_or("");
            row_map.insert(header.clone().into(), Dynamic::from(value.to_string()));
        }

        results.push(Dynamic::from(row_map));
    }

    trace!("Imported {} rows from CSV", results.len());
    Ok(Dynamic::from(results))
}
pub(super) fn import_tsv(file_path: &str) -> Result<Dynamic, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let headers: Vec<String> = match lines.next() {
        Some(Ok(line)) => line.split('\t').map(|s| s.trim().to_string()).collect(),
        _ => return Err("TSV file is empty or has no header".into()),
    };

    let mut results: Array = Array::new();

    for line in lines.map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }

        let values: Vec<String> = line.split('\t').map(|s| s.trim().to_string()).collect();
        let mut row_map: Map = Map::new();

        for (i, header) in headers.iter().enumerate() {
            let value = values.get(i).map(|s| s.as_str()).unwrap_or("");
            row_map.insert(header.clone().into(), Dynamic::from(value.to_string()));
        }

        results.push(Dynamic::from(row_map));
    }

    trace!("Imported {} rows from TSV", results.len());
    Ok(Dynamic::from(results))
}
pub(super) fn import_json(file_path: &str) -> Result<Dynamic, Box<dyn std::error::Error + Send + Sync>> {
    let content = std::fs::read_to_string(file_path)?;
    let json_value: Value = serde_json::from_str(&content)?;

    let result = json_to_dynamic(&json_value);
    trace!("Imported JSON data");
    Ok(result)
}
