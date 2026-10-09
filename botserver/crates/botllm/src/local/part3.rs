use super::*;

pub(super) fn extract_base_url(url: &str) -> String {
    if let Ok(parsed) = url::Url::parse(url) {
        format!(
            "{}://{}{}",
            parsed.scheme(),
            parsed.host_str().unwrap_or("localhost"),
            parsed.port().map(|p| format!(":{}", p)).unwrap_or_default()
        )
    } else {
        url.to_string()
    }
}
pub async fn is_server_running(url: &str) -> bool {
    let base_url = extract_base_url(url);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    match client.get(format!("{base_url}/health")).send().await {
        Ok(response) => {
            if response.status().is_success() {
                return true;
            }

            info!("Health check returned status: {}", response.status());
            false
        }
        Err(e) => match client.get(&base_url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => {
                if !e.is_connect() {
                    warn!("Health check error for {base_url}: {e}");
                }
                false
            }
        },
    }
}
pub fn start_llm_server(
    app_state: Arc<AppState>,
    llama_cpp_path: String,
    model_path: String,
    url: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = extract_port(&url);
    std::env::set_var("OMP_NUM_THREADS", "20");
    std::env::set_var("OMP_PLACES", "cores");
    std::env::set_var("OMP_PROC_BIND", "close");
    let conn = app_state.conn.clone();
    let config_manager = ConfigManager::new(conn.clone());
    let mut conn = conn.get().map_err(|e| {
        Box::new(std::io::Error::other(
            format!("failed to get db connection: {e}"),
        )) as Box<dyn std::error::Error + Send + Sync>
    })?;
    let default_bot_id = bots
        .filter(name.eq("default"))
        .select(id)
        .first::<uuid::Uuid>(&mut *conn)
        .unwrap_or_else(|_| uuid::Uuid::nil());
    let n_moe = config_manager
        .get_config(&default_bot_id, "llm-server-n-moe", None)
        .unwrap_or_else(|_| "4".to_string());
    let n_moe = if n_moe.is_empty() { "4".to_string() } else { n_moe };

    let parallel = config_manager
        .get_config(&default_bot_id, "llm-server-parallel", None)
        .unwrap_or_else(|_| "1".to_string());
    let parallel = if parallel.is_empty() { "1".to_string() } else { parallel };

    let cont_batching = config_manager
        .get_config(&default_bot_id, "llm-server-cont-batching", None)
        .unwrap_or_else(|_| "true".to_string());
    let cont_batching = if cont_batching.is_empty() { "true".to_string() } else { cont_batching };

    let mlock = config_manager
        .get_config(&default_bot_id, "llm-server-mlock", None)
        .unwrap_or_else(|_| "true".to_string());
    let mlock = if mlock.is_empty() { "true".to_string() } else { mlock };

    let no_mmap = config_manager
        .get_config(&default_bot_id, "llm-server-no-mmap", None)
        .unwrap_or_else(|_| "true".to_string());
    let no_mmap = if no_mmap.is_empty() { "true".to_string() } else { no_mmap };

    let gpu_layers = config_manager
        .get_config(&default_bot_id, "llm-server-gpu-layers", None)
        .unwrap_or_else(|_| "0".to_string());
    let gpu_layers = if gpu_layers.is_empty() { "0".to_string() } else { gpu_layers };

    let reasoning_format = config_manager
        .get_config(&default_bot_id, "llm-server-reasoning-format", None)
        .unwrap_or_else(|_| String::new());

    let n_predict = config_manager
        .get_config(&default_bot_id, "llm-server-n-predict", None)
        .unwrap_or_else(|_| "512".to_string());  // Increased default for DeepSeek R1 reasoning
    let n_predict = if n_predict.is_empty() { "512".to_string() } else { n_predict };

    let n_ctx_size = config_manager
        .get_config(&default_bot_id, "llm-server-ctx-size", None)
        .unwrap_or_else(|_| "32000".to_string());
    let n_ctx_size = if n_ctx_size.is_empty() { "32000".to_string() } else { n_ctx_size };

    let cmd_path = if cfg!(windows) {
        format!("{}\\llama-server.exe", llama_cpp_path)
    } else {
        format!("{}/llama-server", llama_cpp_path)
    };

    // Get ubatch-size from config, default to 512 if not set
    let ubatch_size = config_manager
        .get_config(&default_bot_id, "llm-server-ubatch-size", Some("512"))
        .unwrap_or_else(|_| "512".to_string());
    let ubatch_size = if ubatch_size.is_empty() { "512".to_string() } else { ubatch_size };

    let mut args_vec = vec![
        "-m", &model_path,
        "--host", "127.0.0.1",
        "--port", port,
        "--top_p", "0.95",
        "--temp", "0.6",
        "--repeat-penalty", "1.2",
        "--n-gpu-layers", &gpu_layers,
        "--ubatch-size", &ubatch_size,
    ];

    if !reasoning_format.is_empty() {
        args_vec.push("--reasoning-format");
        args_vec.push(&reasoning_format);
    }
    if n_moe != "0" {
        args_vec.push("--n-cpu-moe");
        args_vec.push(&n_moe);
    }
    if parallel != "1" {
        args_vec.push("--parallel");
        args_vec.push(&parallel);
    }
    if cont_batching == "true" {
        args_vec.push("--cont-batching");
    }
    if mlock == "true" {
        args_vec.push("--mlock");
    }
    if no_mmap == "true" {
        args_vec.push("--no-mmap");
    }
    if n_predict != "0" {
        args_vec.push("--n-predict");
        args_vec.push(&n_predict);
    }
    args_vec.push("--ctx-size");
    args_vec.push(&n_ctx_size);
    args_vec.push("--verbose");

    let mut command = SafeCommand::new(&cmd_path)?;
    command = command.args(&args_vec)?;
    command = command.working_dir(std::path::Path::new(&llama_cpp_path))?;
    command = command.env("LD_LIBRARY_PATH", &llama_cpp_path)?;

    let log_file_path = if cfg!(windows) {
        format!("{}\\llm-stdout.log", llama_cpp_path)
    } else {
        format!("{}/llm-stdout.log", llama_cpp_path)
    };

    match std::fs::File::create(&log_file_path) {
        Ok(log_file) => {
            if let Ok(clone) = log_file.try_clone() {
                command = command.stdout(std::process::Stdio::from(clone));
            } else {
                command = command.stdout(std::process::Stdio::null());
            }
            command = command.stderr(std::process::Stdio::from(log_file));
        }
        Err(_) => {
            command = command.stdout(std::process::Stdio::null());
            command = command.stderr(std::process::Stdio::null());
        }
    }

    info!("Executing LLM server command: llama-server with {} args", args_vec.len());
    
    command.spawn().map_err(|e| {
        Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>
    })?;
    Ok(())
}
pub async fn start_embedding_server(
    llama_cpp_path: String,
    model_path: String,
    url: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = extract_port(&url);

    // model_path is already the full path (constructed with ../../../../data/llm/ prefix)
    // Only prepend llama_cpp_path if model_path is a simple filename (not a path)
    let full_model_path = if model_path.contains('/') || model_path.contains('.') {
        // model_path is already a full or relative path, use as-is
        model_path.clone()
    } else {
        // model_path is just a filename, prepend llama_cpp_path
        format!("{llama_cpp_path}/{model_path}")
    };

    if !std::path::Path::new(&full_model_path).exists() {
        log::warn!(
            "Embedding model '{model_path}' is not installed at {full_model_path}; local embedding server not started"
        );
        return Err(format!("Embedding model not installed: {full_model_path}").into());
    }

    info!("Starting embedding server on port {port} with model: {model_path}");

    let _cmd_path = if cfg!(windows) {
        format!("{}\\llama-server.exe", llama_cpp_path)
    } else {
        format!("{}/llama-server", llama_cpp_path)
    };

    let mut args_vec = vec![
        "-m", &full_model_path,
        "--host", "127.0.0.1",
        "--port", port,
        "--embeddings",
        "--pooling", "mean",
        "--n-gpu-layers", "0",
        "--ctx-size", "512",
    ];

    if !cfg!(windows) {
        args_vec.push("--ubatch-size");
        args_vec.push("512");
    }

    let cmd_path = if cfg!(windows) {
        format!("{}\\llama-server.exe", llama_cpp_path)
    } else {
        format!("{}/llama-server", llama_cpp_path)
    };

    let mut command = SafeCommand::new(&cmd_path)?;
    command = command.args(&args_vec)?;
    command = command.working_dir(std::path::Path::new(&llama_cpp_path))?;
    command = command.env("LD_LIBRARY_PATH", &llama_cpp_path)?;

    let log_file_path = if cfg!(windows) {
        format!("{}\\stdout.log", llama_cpp_path)
    } else {
        format!("{}/llmembd-stdout.log", llama_cpp_path)
    };

    match std::fs::File::create(&log_file_path) {
        Ok(log_file) => {
            if let Ok(clone) = log_file.try_clone() {
                command = command.stdout(std::process::Stdio::from(clone));
            } else {
                command = command.stdout(std::process::Stdio::null());
            }
            command = command.stderr(std::process::Stdio::from(log_file));
        }
        Err(_) => {
            command = command.stdout(std::process::Stdio::null());
            command = command.stderr(std::process::Stdio::null());
        }
    }

    info!("Executing embedding server command: llama-server with {} args", args_vec.len());
    
    command.spawn().map_err(|e| {
        Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error + Send + Sync>
    })?;

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}
pub(super) fn extract_port(url: &str) -> &str {
    url.rsplit(':')
        .next()
        .unwrap_or("8081")
        .split('/')
        .next()
        .unwrap_or("8081")
}
