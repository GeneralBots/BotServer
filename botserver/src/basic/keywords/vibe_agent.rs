//!  #751 — VIBE RUN keyword and friends (VIBE STATUS / VIBE APPROVE /
//! VIBE CANCEL / VIBE TOOLS / VIBE EVENTS).
//!
//! Bridges the BASIC chat surface to the Vibe agent API running on the
//! local botserver. Every keyword performs a single HTTP call through a
//! short-lived thread + runtime, mirroring the pattern used by the other
//! botserver-local keywords (set_answer_mode, send_mail).
//!
//! The base URL comes from `VIBE_API_URL` (default `http://localhost:8080`).

use rhai::{Dynamic, Engine};

const DEFAULT_VIBE_URL: &str = "http://localhost:8080";

fn vibe_base_url() -> String {
    let (_, vibe) = botcoresecrets::app_runtime();
    if vibe.is_empty() {
        std::env::var("VIBE_API_URL").unwrap_or_else(|_| DEFAULT_VIBE_URL.to_string())
    } else {
        vibe
    }
}

fn http_json(
    method: &'static str,
    path_and_query: String,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", vibe_base_url(), path_and_query);
    let internal_token = botcoresecrets::internal_api_token();
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(Err(format!("runtime: {e}")));
                return;
            }
        };
        let result = rt.block_on(async move {
            let client = reqwest::Client::new();
            let mut builder = match method {
                "GET" => client.get(&url),
                "POST" => client.post(&url),
                "PUT" => client.put(&url),
                "DELETE" => client.delete(&url),
                _ => return Err(format!("unsupported method {method}")),
            };
            if !internal_token.is_empty() {
                builder = builder.header("X-Internal-Token", internal_token);
            }
            let builder = if let Some(b) = body {
                builder.json(&b)
            } else {
                builder
            };
            let resp = match builder.send().await {
                Ok(r) => r,
                Err(e) => return Err(format!("Vibe API request failed: {e}")),
            };
            let status = resp.status();
            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => return Err(format!("read response: {e}")),
            };
            if !status.is_success() {
                return Err(format!("Vibe API returned {status}: {text}"));
            }
            serde_json::from_str::<serde_json::Value>(&text)
                .map_err(|e| format!("parse response: {e}"))
        });
        let _ = tx.send(result);
    });

    match rx.recv() {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(format!("channel error: {e}")),
    }
}

/// `VIBE RUN "{intent}"` — creates an agent run in the Vibe subsystem and
/// returns the run id plus the resolved use case.
///
/// Registered as a function (`vibe_run`) because Rhai keys
/// `register_custom_syntax` by the FIRST token: six variants sharing the
/// "VIBE" prefix would overwrite each other (only the last registration
/// would ever match). The BASIC preprocessor rewrites the `VIBE RUN` /
/// `VIBE STATUS` / ... forms into these function calls.
pub fn register_vibe_run_keyword(engine: &mut Engine) {
    engine.register_fn("vibe_run", |intent: String| -> Dynamic {
        let payload = serde_json::json!({
            "intent": intent,
            // The keyword IS the operator's instruction: tools execute
            // without an interactive approval round-trip (the run would
            // otherwise idle for its 300s approval window and time out).
            "auto_approve": true,
        });
        match http_json("POST", "/api/vibe/run".into(), Some(payload)) {
            Ok(v) => {
                let run_id = v.get("run_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let state = v.get("state").and_then(|x| x.as_str()).unwrap_or("").to_string();
                log::info!("VIBE RUN created run_id={run_id} state={state}");
                Dynamic::from(format!(
                    "Vibe run created: {run_id} (state: {state})"
                ))
            }
            Err(e) => Dynamic::from(format!("VIBE RUN failed: {e}")),
        }
    });
}

/// `VIBE STATUS "{run_id}"` — read the current run state and tool results.
pub fn register_vibe_status_command(engine: &mut Engine) {
    engine.register_fn("vibe_status", |run_id: String| -> Dynamic {
        match http_json("GET", format!("/api/vibe/run/{run_id}"), None) {
            Ok(v) => {
                let state = v.get("state").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let tools = v.get("tool_call_count").and_then(|x| x.as_u64()).unwrap_or(0);
                let error = v.get("error").and_then(|x| x.as_str()).unwrap_or("");
                Dynamic::from(format!(
                    "Vibe run {run_id}: state={state}, tools_executed={tools}{}",
                    if error.is_empty() { String::new() } else { format!(", error={error}") }
                ))
            }
            Err(e) => Dynamic::from(format!("VIBE STATUS failed: {e}")),
        }
    });
}

/// `VIBE APPROVE "{run_id}"` — approve pending tool calls of a run.
pub fn register_vibe_approve_command(engine: &mut Engine) {
    engine.register_fn("vibe_approve", |run_id: String| -> Dynamic {
        match http_json("POST", format!("/api/vibe/run/{run_id}/approve"), None) {
            Ok(v) => {
                let approved = v.get("approved").and_then(|x| x.as_bool())
                    .or_else(|| v.get("success").and_then(|x| x.as_bool()))
                    .unwrap_or(false);
                let msg = v.get("message").and_then(|x| x.as_str()).unwrap_or("");
                Dynamic::from(format!(
                    "Vibe approval {}: {}",
                    if approved { "accepted" } else { "rejected" },
                    msg
                ))
            }
            Err(e) => Dynamic::from(format!("VIBE APPROVE failed: {e}")),
        }
    });
}

/// `VIBE CANCEL "{run_id}"` — cancel a running or awaiting-approval run.
pub fn register_vibe_cancel_command(engine: &mut Engine) {
    engine.register_fn("vibe_cancel", |run_id: String| -> Dynamic {
        match http_json("POST", format!("/api/vibe/run/{run_id}/cancel"), None) {
            Ok(v) => {
                let ok = v.get("success").and_then(|x| x.as_bool()).unwrap_or(false);
                let msg = v.get("message").and_then(|x| x.as_str()).unwrap_or("");
                Dynamic::from(format!(
                    "Vibe cancel {}: {}",
                    if ok { "accepted" } else { "rejected" },
                    msg
                ))
            }
            Err(e) => Dynamic::from(format!("VIBE CANCEL failed: {e}")),
        }
    });
}

/// `VIBE TOOLS` — list the tools the agent can use.
pub fn register_vibe_tools_command(engine: &mut Engine) {
    engine.register_fn("vibe_tools", || -> Dynamic {
        match http_json("GET", "/api/vibe/tools".into(), None) {
            Ok(v) => {
                if let Some(tools) = v.get("tools").and_then(|x| x.as_array()) {
                    let names: Vec<&str> = tools
                        .iter()
                        .filter_map(|t| {
                            t.get("schema").and_then(|s| s.get("name")).and_then(|n| n.as_str())
                        })
                        .collect();
                    Dynamic::from(format!(
                        "Available Vibe tools ({}): {}",
                        names.len(),
                        names.join(", ")
                    ))
                } else {
                    Dynamic::from("No Vibe tools reported.")
                }
            }
            Err(e) => Dynamic::from(format!("VIBE TOOLS failed: {e}")),
        }
    });
}

/// `VIBE EVENTS "{run_id}"` — fetch the latest progress events of a run.
pub fn register_vibe_events_command(engine: &mut Engine) {
    engine.register_fn("vibe_events", |run_id: String| -> Dynamic {
        match http_json("GET", format!("/api/vibe/events/{run_id}"), None) {
            Ok(v) => {
                if let Some(events) = v.as_array() {
                    let summary: Vec<String> = events
                        .iter()
                        .map(|e| {
                            let step = e.get("step").and_then(|x| x.as_str()).unwrap_or("?");
                            let msg = e.get("message").and_then(|x| x.as_str()).unwrap_or("");
                            format!("[{step}] {msg}")
                        })
                        .collect();
                    Dynamic::from(format!(
                        "Vibe events ({}): {}",
                        summary.len(),
                        summary.join(" | ")
                    ))
                } else {
                    Dynamic::from("No Vibe events yet.")
                }
            }
            Err(e) => Dynamic::from(format!("VIBE EVENTS failed: {e}")),
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> Engine {
        let mut engine = Engine::new();
        register_vibe_run_keyword(&mut engine);
        register_vibe_status_command(&mut engine);
        register_vibe_approve_command(&mut engine);
        register_vibe_cancel_command(&mut engine);
        register_vibe_tools_command(&mut engine);
        register_vibe_events_command(&mut engine);
        engine
    }

    #[test]
    fn vibe_run_keyword_registers_and_parses() {
        let engine = make_engine();
        engine
            .compile(r#"vibe_run("deploy the app")"#)
            .expect("vibe_run() should parse");
    }

    #[test]
    fn vibe_status_parses_with_string_arg() {
        let engine = make_engine();
        engine
            .compile(r#"vibe_status("run-123")"#)
            .expect("vibe_status() should parse");
    }

    #[test]
    fn vibe_approve_and_cancel_parse() {
        let engine = make_engine();
        engine.compile(r#"vibe_approve("run-123")"#).expect("vibe_approve() should parse");
        engine.compile(r#"vibe_cancel("run-123")"#).expect("vibe_cancel() should parse");
    }

    #[test]
    fn vibe_tools_parses_without_arguments() {
        let engine = make_engine();
        engine.compile("vibe_tools()").expect("vibe_tools() should parse");
    }

    #[test]
    fn vibe_events_parses_with_run_id() {
        let engine = make_engine();
        engine.compile(r#"vibe_events("run-123")"#).expect("vibe_events() should parse");
    }

    #[test]
    fn keyword_forms_are_rewritten_by_preprocessor() {
        use botbasic_compiler::syntax_transforms::convert_multiword_keywords;
        let out = convert_multiword_keywords(
            r#"VIBE RUN "deploy the app"
VIBE STATUS "run-123"
VIBE APPROVE "run-123"
VIBE CANCEL "run-123"
VIBE EVENTS "run-123"
VIBE TOOLS"#,
        );
        assert!(out.contains(r#"vibe_run("deploy the app")"#), "got: {out}");
        assert!(out.contains(r#"vibe_status("run-123")"#), "got: {out}");
        assert!(out.contains(r#"vibe_approve("run-123")"#), "got: {out}");
        assert!(out.contains(r#"vibe_cancel("run-123")"#), "got: {out}");
        assert!(out.contains(r#"vibe_events("run-123")"#), "got: {out}");
        assert!(out.contains("vibe_tools()"), "got: {out}");
    }

    #[test]
    fn preprocessor_strips_ast_trailing_semicolon() {
        use botbasic_compiler::syntax_transforms::convert_multiword_keywords;
        let out = convert_multiword_keywords(
            r#"VIBE RUN "deploy the app";
VIBE TOOLS;"#,
        );
        assert!(out.contains(r#"vibe_run("deploy the app");"#), "got: {out}");
        assert!(out.contains("vibe_tools();"), "got: {out}");
        assert!(!out.contains(r#"vibe_run("deploy the app";"#), "semicolon leaked into args: {out}");
        // quoted semicolons must be preserved
        let quoted = convert_multiword_keywords(r#"VIBE RUN "add ; a semicolon";"#);
        assert!(quoted.contains(r#"vibe_run("add ; a semicolon");"#), "got: {quoted}");
    }
}
