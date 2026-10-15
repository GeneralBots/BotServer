use super::*;

pub async fn ensure_llama_servers_running(
    app_state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if LLAMA_SERVERS_STARTED.swap(true, Ordering::SeqCst) {
        info!("ensure_llama_servers_running already called, skipping duplicate invocation");
        return Ok(());
    }
    trace!("ensure_llama_servers_running ENTER");
    let start_mem = MemoryStats::current();
    trace!(
        "[LLM_LOCAL] ensure_llama_servers_running START, RSS={}",
        MemoryStats::format_bytes(start_mem.rss_bytes)
    );
    log_jemalloc_stats();

    if std::env::var("SKIP_LLM_SERVER").is_ok() {
        trace!("SKIP_LLM_SERVER set, returning early");
        info!("SKIP_LLM_SERVER set - skipping local LLM server startup (using mock/external LLM)");
        return Ok(());
    }

    let config_values = {
        let conn_arc = app_state.conn.clone();
        let (default_bot_id, _default_bot_name) = tokio::task::spawn_blocking(move || -> Result<(uuid::Uuid, String), String> {
            let mut conn = conn_arc
                .get()
                .map_err(|e| format!("failed to get db connection: {e}"))?;
            Ok(botcore::bot::get_default_bot(&mut conn))
        })
        .await??;
        let config_manager = ConfigManager::new(app_state.conn.clone());
        info!("Reading config for bot_id: {}", default_bot_id);
        let embedding_model_result = config_manager.get_config(&default_bot_id, "embedding-model", None);
        info!("embedding-model config result: {:?}", embedding_model_result);
        (
            default_bot_id,
            config_manager
                .get_config(&default_bot_id, "llm-server", Some("true"))
                .unwrap_or_else(|_| "true".to_string()),
            config_manager
                .get_config(&default_bot_id, "llm-url", Some(""))
                .unwrap_or_else(|_| "".to_string()),
            config_manager
                .get_config(&default_bot_id, "llm-model", None)
                .unwrap_or_default(),
            config_manager
                .get_config(&default_bot_id, "embedding-url", Some(""))
                .unwrap_or_else(|_| "".to_string()),
            embedding_model_result.unwrap_or_default(),
            config_manager
                .get_config(&default_bot_id, "llm-server-path", None)
                .unwrap_or_default(),
        )
    };
    let (
        default_bot_id,
        llm_server_enabled,
        llm_url,
        llm_model,
        embedding_url,
        embedding_model,
        llm_server_path,
    ) = config_values;

    let llm_server_enabled = llm_server_enabled.to_lowercase() == "true";

    // Use default models when config is empty (no default.gbai/config.csv)
    let llm_server_path = if llm_server_path.is_empty() {
        format!("{}/bin/llm/bin", botcore::shared::utils::get_stack_path())
    } else {
        llm_server_path
    };

let llm_url = if llm_url.is_empty() && llm_server_enabled {
    let url = "http://localhost:8081/v1/chat/completions".to_string();
    info!("No llm-url configured with local server enabled, using default: {url}");
    let config_manager = ConfigManager::new(app_state.conn.clone());
    if let Err(e) = config_manager.set_config(&default_bot_id, "llm-url", &url) {
        warn!("Failed to persist default llm-url: {e}");
    }
    url
} else {
    llm_url
};

    // Use config values, fallback to safe defaults for local development
    let llm_model = if llm_model.is_empty() {
        info!("No LLM model configured, using default: DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf");
        "DeepSeek-R1-Distill-Qwen-1.5B-Q3_K_M.gguf".to_string()
    } else {
        llm_model
    };

    let embedding_model = if embedding_model.is_empty() {
        info!("No embedding model configured, using default: bge-small-en-v1.5-f32.gguf");
        "bge-small-en-v1.5-f32.gguf".to_string()
    } else {
        embedding_model
    };

    let embedding_url = if embedding_url.is_empty() {
        let default_port = "8082";
        let url = format!("http://localhost:{default_port}/v1/embeddings");
        info!("No embedding-url configured, using default: {url}");
        url
    } else {
        embedding_url
    };

    // For llama-server startup, use path relative to botserver root
    // The models are in <stack_path>/data/llm/ and the llama-server runs from botserver root.
    // Template config.csv files may carry a legacy relative path (e.g.
    // "../../../../data/llm/model.gguf"); reduce it to the bare filename and
    // rejoin under the stack data dir so the path resolves on any layout.
    let stack_path = botcore::shared::utils::get_stack_path();
    let llm_model_path = format!("{stack_path}/data/llm/{}", model_file_name(&llm_model));
    let embedding_model_path = format!("{stack_path}/data/llm/{}", model_file_name(&embedding_model));
    if !llm_server_enabled {
        info!("Local LLM server management disabled (llm-server=false). Using external endpoints.");
        info!("  LLM URL: {llm_url}");
        info!("  Embedding URL: {embedding_url}");
        return Ok(());
    }
    info!("Starting LLM servers...");
    info!("Configuration:");
    info!("  LLM URL: {llm_url}");
    info!("  Embedding URL: {embedding_url}");
    info!("  LLM Model: {llm_model}");
    info!("  Embedding Model: {embedding_model}");
    info!("  LLM Server Path: {llm_server_path}");
    let llm_running = if llm_url.starts_with("https://") {
        info!("Using external HTTPS LLM server, skipping local startup");
        true
    } else {
        is_server_running(&llm_url).await
    };

    let embedding_running = if embedding_url.starts_with("https://") {
        info!("Using external HTTPS embedding server, skipping local startup");
        true
    } else {
        is_server_running(&embedding_url).await
    };
    if llm_running && embedding_running {
        info!("Both LLM and Embedding servers are already running");
        if !embedding_model.is_empty() {
            set_embedding_server_ready(true);
        }
        return Ok(());
    }

    info!("Killing existing llama-server processes to restart with correct args...");
    let pkill_result = SafeCommand::new("sh")
        .and_then(|c| c.arg("-c"))
        .and_then(|c| c.trusted_shell_script_arg("pkill llama-server -9; true"));

    match pkill_result {
        Ok(cmd) => {
            if let Err(e) = cmd.execute() {
                log::error!("Failed to execute pkill for llama-server: {e}");
            } else {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                info!("Existing llama-server processes terminated");
            }
        }
        Err(e) => log::error!("Failed to build pkill command: {e}"),
    }
    let mut tasks = vec![];
    if !llm_running && !llm_model.is_empty() {
        info!("Starting LLM server...");
        let app_state_clone = Arc::clone(&app_state);
        let llm_server_path_clone = llm_server_path.clone();
        let llm_model_path_clone = llm_model_path.clone();
        let llm_url_clone = llm_url.clone();
        tasks.push(tokio::spawn(async move {
            start_llm_server(
                app_state_clone,
                llm_server_path_clone,
                llm_model_path_clone,
                llm_url_clone,
            )
        }));
    } else if llm_model.is_empty() {
        info!("LLM_MODEL not set, skipping LLM server");
    }
    if !embedding_running && !embedding_model.is_empty() {
        // The local llama-server needs the GGUF model on disk. Deployments that
        // embed through a remote provider (Cloudflare/OpenAI-compatible) never
        // install it, so a missing file is an expected configuration, not a
        // failure: skip the spawn instead of logging an error on every boot.
        if std::path::Path::new(&embedding_model_path).exists() {
            info!("Starting Embedding server...");
            tasks.push(tokio::spawn(start_embedding_server(
                llm_server_path.clone(),
                embedding_model_path.clone(),
                embedding_url.clone(),
            )));
        } else {
            warn!(
                "Embedding model '{embedding_model}' is not installed at {embedding_model_path}; skipping the local embedding server (embeddings use the configured remote provider)"
            );
        }
    } else if embedding_model.is_empty() {
        info!("EMBEDDING_MODEL not set, skipping Embedding server");
    }
    // Start servers in background - don't block HTTP server startup
    if !tasks.is_empty() {
        info!("LLM servers starting in background (non-blocking mode)");
        tokio::spawn(async move {
            for task in tasks {
                if let Err(e) = task.await {
                    log::error!("LLM server task failed: {}", e);
                }
            }
            info!("LLM server startup tasks completed");
        });
    }

    // Return immediately - don't wait for servers to be ready
    info!("LLM server initialization initiated (will start in background)");
    info!("HTTP server can start without waiting for LLM servers");
    trace!("ensure_llama_servers_running returning early (non-blocking)");

    let end_mem = MemoryStats::current();
    trace!(
        "[LLM_LOCAL] ensure_llama_servers_running END (non-blocking), RSS={} (total delta={})",
        MemoryStats::format_bytes(end_mem.rss_bytes),
        MemoryStats::format_bytes(end_mem.rss_bytes.saturating_sub(start_mem.rss_bytes))
    );
    log_jemalloc_stats();

    trace!("ensure_llama_servers_running EXIT OK (non-blocking)");
    Ok(())

    // OLD BLOCKING CODE - REMOVED TO PREVENT HTTP SERVER BLOCKING
    /*
    for task in tasks {
        task.await??;
    }
    info!("Waiting for servers to become ready...");
    trace!("Starting wait loop for servers...");
    let before_wait = MemoryStats::current();
    trace!(
        "[LLM_LOCAL] Before wait loop, RSS={}",
        MemoryStats::format_bytes(before_wait.rss_bytes)
    );

    let mut llm_ready = llm_running || llm_model.is_empty();
    let mut embedding_ready = embedding_running || embedding_model.is_empty();
    let mut attempts = 0;
    let max_attempts = 15; // Reduced from 120 to 15 (30 seconds instead of 240)
    while attempts < max_attempts && (!llm_ready || !embedding_ready) {
        trace!("Wait loop iteration {}", attempts);
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        if attempts % 5 == 0 {
            let loop_mem = MemoryStats::current();
            trace!(
                "[LLM_LOCAL] Wait loop attempt {}, RSS={} (delta from start={})",
                attempts,
                MemoryStats::format_bytes(loop_mem.rss_bytes),
                MemoryStats::format_bytes(loop_mem.rss_bytes.saturating_sub(before_wait.rss_bytes))
            );
            log_jemalloc_stats();
        }

        if attempts % 5 == 0 {
            info!(
                "Checking server health (attempt {}/{max_attempts})...",
                attempts + 1
            );
        }
        if !llm_ready && !llm_model.is_empty() {
            if is_server_running(&llm_url).await {
                info!("LLM server ready at {llm_url}");
                llm_ready = true;
            } else {
                info!("LLM server not ready yet");
            }
        }
        if !embedding_ready && !embedding_model.is_empty() {
            if is_server_running(&embedding_url).await {
                info!("Embedding server ready at {embedding_url}");
                embedding_ready = true;
                set_embedding_server_ready(true);
            } else if attempts % 10 == 0 {
                warn!("Embedding server not ready yet at {embedding_url}");

                if let Ok(log_content) =
                    std::fs::read_to_string(format!("{llm_server_path}/llmembd-stdout.log"))
                {
                    let last_lines: Vec<&str> = log_content.lines().rev().take(5).collect();
                    if !last_lines.is_empty() {
                        info!("Embedding server log (last 5 lines):");
                        for line in last_lines.iter().rev() {
                            info!("  {line}");
                        }
                    }
                }
            }
        }
        attempts += 1;
        if attempts % 20 == 0 {
            warn!(
                "Still waiting for servers... (attempt {attempts}/{max_attempts}) - this may take a while for large models"
            );
        }
    }
    if llm_ready && embedding_ready {
        info!("All llama.cpp servers are ready and responding!");
        if !embedding_model.is_empty() {
            set_embedding_server_ready(true);
        }
        trace!("Servers ready!");

        let after_ready = MemoryStats::current();
        trace!(
            "[LLM_LOCAL] Servers ready, RSS={} (delta from start={})",
            MemoryStats::format_bytes(after_ready.rss_bytes),
            MemoryStats::format_bytes(after_ready.rss_bytes.saturating_sub(start_mem.rss_bytes))
        );
        log_jemalloc_stats();

        let _llm_provider1 = Arc::new(crate::openai_client::OpenAIClient::new(
            llm_model.clone(),
            Some(llm_url.clone()),
            None,
        ));

        let end_mem = MemoryStats::current();
        trace!(
            "[LLM_LOCAL] ensure_llama_servers_running END, RSS={} (total delta={})",
            MemoryStats::format_bytes(end_mem.rss_bytes),
            MemoryStats::format_bytes(end_mem.rss_bytes.saturating_sub(start_mem.rss_bytes))
        );
        log_jemalloc_stats();

        trace!("ensure_llama_servers_running EXIT OK");
        Ok(())
    } else {
        let mut error_msg = "Servers failed to start within timeout:".to_string();
        if !llm_ready && !llm_model.is_empty() {
            let _ = write!(error_msg, "\n   - LLM server at {llm_url}");
        }
        if !embedding_ready && !embedding_model.is_empty() {
            let _ = write!(error_msg, "\n   - Embedding server at {embedding_url}");
        }
        Err(error_msg.into())
    }
    */ // END OF OLD BLOCKING CODE
}
