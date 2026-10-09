use super::*;

pub fn register_set_header_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let headers: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let headers_clone = Arc::clone(&headers);
    let headers_clone2 = Arc::clone(&headers);

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "HEADER", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let name = context.eval_expression_tree(&inputs[0])?.to_string();
                let value = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("SET HEADER: {name} = {value}");

                HTTP_HEADERS.with(|h| {
                    h.borrow_mut().insert(name.clone(), value.clone());
                });

                if let Ok(mut h) = headers_clone.lock() {
                    h.insert(name, value);
                }

                Ok(Dynamic::UNIT)
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET_HEADER", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let name = context.eval_expression_tree(&inputs[0])?.to_string();
                let value = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("SET_HEADER: {name} = {value}");

                HTTP_HEADERS.with(|h| {
                    h.borrow_mut().insert(name.clone(), value.clone());
                });

                if let Ok(mut h) = headers_clone2.lock() {
                    h.insert(name, value);
                }

                Ok(Dynamic::UNIT)
            },
        )
        {
            log::error!("Failed to register the custom syntax: {e}");
        }
}
pub fn register_clear_headers_keyword(
    _state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    if let Err(e) = engine
        .register_custom_syntax(["CLEAR", "HEADERS"], false, move |_context, _inputs| {
            trace!("CLEAR HEADERS");

            HTTP_HEADERS.with(|h| {
                h.borrow_mut().clear();
            });

            Ok(Dynamic::UNIT)
        })
{
    log::error!("Failed to register the custom syntax: {e}");
}

    if let Err(e) = engine
        .register_custom_syntax(["CLEAR_HEADERS"], false, move |_context, _inputs| {
            trace!("CLEAR_HEADERS");

            HTTP_HEADERS.with(|h| {
                h.borrow_mut().clear();
            });

            Ok(Dynamic::UNIT)
        })
        {
            log::error!("Failed to register the custom syntax: {e}");
        }
}
pub fn register_graphql_keyword(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GRAPHQL", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let endpoint = context.eval_expression_tree(&inputs[0])?.to_string();
                let query = context.eval_expression_tree(&inputs[1])?.to_string();
                let variables = context.eval_expression_tree(&inputs[2])?;

                trace!("GRAPHQL request to: {endpoint}");

                let (tx, rx) = std::sync::mpsc::channel();
                let endpoint_clone = endpoint;
                let query_clone = query;
                let variables_json = dynamic_to_json(&variables);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_graphql(&endpoint_clone, &query_clone, variables_json).await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send GRAPHQL result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(60)) {
                    Ok(Ok(response)) => Ok(json_to_dynamic(&response)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("GRAPHQL failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "GRAPHQL request timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("GRAPHQL thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_soap_keyword(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["SOAP", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let wsdl = context.eval_expression_tree(&inputs[0])?.to_string();
                let operation = context.eval_expression_tree(&inputs[1])?.to_string();
                let params = context.eval_expression_tree(&inputs[2])?;

                trace!("SOAP request to: {wsdl}, operation: {operation}");

                let (tx, rx) = std::sync::mpsc::channel();
                let wsdl_clone = wsdl;
                let operation_clone = operation;
                let params_json = dynamic_to_json(&params);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            execute_soap(&wsdl_clone, &operation_clone, params_json).await
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".into())).err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send SOAP result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(120)) {
                    Ok(Ok(response)) => Ok(json_to_dynamic(&response)),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SOAP failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "SOAP request timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("SOAP thread failed: {e}").into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub(super) async fn execute_http_request(
    method: Method,
    url: &str,
    body: Option<Value>,
    custom_headers: Option<HashMap<String, String>>,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| {
            log::error!("Failed to build HTTP client: {e}");
            e
        })?;

    let mut headers = HeaderMap::new();

    HTTP_HEADERS.with(|h| {
        for (name, value) in h.borrow().iter() {
            if let (Ok(header_name), Ok(header_value)) = (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                headers.insert(header_name, header_value);
            }
        }
    });

    if let Some(custom) = custom_headers {
        for (name, value) in custom {
            if let (Ok(header_name), Ok(header_value)) = (
                HeaderName::try_from(name.as_str()),
                HeaderValue::try_from(value.as_str()),
            ) {
                headers.insert(header_name, header_value);
            }
        }
    }

    if body.is_some() && !headers.contains_key("content-type") {
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }

    let mut request = client.request(method.clone(), url).headers(headers);

    if let Some(body_data) = body {
        request = request.json(&body_data);
    }

    let response = request.send().await.map_err(|e| {
        log::error!("HTTP {method} request failed for URL {url}: {e}");
        e
    })?;

    let status = response.status();
    let response_headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body_text = response.text().await.unwrap_or_default();

    let body_value: Value = serde_json::from_str(&body_text).unwrap_or(Value::String(body_text));

    trace!("HTTP {method} request to {url} completed with status: {status}");

    Ok(json!({
        "status": status.as_u16(),
        "statusText": status.canonical_reason().unwrap_or(""),
        "headers": response_headers,
        "data": body_value
    }))
}
