//! `agent_loop::tests` — split per #1443.

use super::*;

    use uuid::Uuid;
    use super::llm_5::project_scoped_arguments;
    use crate::types::VibeRunConfig;
    use serde_json::json;
    use tokio::sync::RwLock;

    #[test]
    fn classifies_non_retryable_llm_errors() {
        assert!(AgentLoop::is_non_retryable_llm_error(
            "LLM returned status 401: unauthorized"
        ));
        assert!(AgentLoop::is_non_retryable_llm_error(
            "LLM returned status 403: forbidden"
        ));
        assert!(AgentLoop::is_non_retryable_llm_error(
            "LLM returned status 404: model not found"
        ));
        assert!(AgentLoop::is_non_retryable_llm_error(
            "LLM returned status 422: bad request"
        ));
        assert!(!AgentLoop::is_non_retryable_llm_error(
            "LLM returned status 429: rate limited"
        ));
        assert!(!AgentLoop::is_non_retryable_llm_error(
            "LLM returned status 408: timeout"
        ));
        assert!(!AgentLoop::is_non_retryable_llm_error(
            "LLM returned status 502: bad gateway"
        ));
        assert!(!AgentLoop::is_non_retryable_llm_error(
            "HTTP request failed: connection reset"
        ));
    }

    #[test]
    fn local_windows_requires_the_initial_registered_tool_call() {
        let mut run = VibeRun::new(
            Uuid::nil(),
            Uuid::nil(),
            Uuid::nil(),
            "Change the page title".to_string(),
            VibeRunConfig::default(),
        );
        let choice =
            AgentLoop::local_forced_tool("http://localhost:8081/v1/chat/completions", &run);
        assert_eq!(
            choice,
            if cfg!(windows) {
                Some("file/set-title")
            } else {
                None
            }
        );
        run.tool_calls.push(VibeToolCall::new(
            run.run_id,
            "file/set-title".to_string(),
            json!({}),
            true,
        ));
        let next = AgentLoop::local_forced_tool("http://localhost:8081/v1/chat/completions", &run);
        assert_eq!(next, None);
        assert_eq!(
            AgentLoop::local_forced_tool("https://api.openai.com/v1/chat/completions", &run),
            None
        );
    }

    #[test]
    fn local_windows_uses_focused_replace_for_existing_file_edits() {
        let mut run = VibeRun::new(
            Uuid::nil(),
            Uuid::nil(),
            Uuid::nil(),
            "Blue theme for the calculator".to_string(),
            VibeRunConfig::default(),
        );
        let api_url = "http://localhost:8081/v1/chat/completions";
        let expected = if cfg!(windows) {
            Some("file/list")
        } else {
            None
        };
        assert_eq!(AgentLoop::local_forced_tool(api_url, &run), expected);
        if !cfg!(windows) {
            return;
        }

        for (completed, next) in [
            ("file/list", Some("file/read")),
            ("file/read", Some("file/replace")),
            ("file/replace", None),
        ] {
            run.tool_calls.push(VibeToolCall::new(
                run.run_id,
                completed.to_string(),
                json!({}),
                false,
            ));
            assert_eq!(AgentLoop::local_forced_tool(api_url, &run), next);
        }
    }

    #[test]
    fn local_windows_requires_publish_tool_for_deployment_intent() {
        let run = VibeRun::new(
            Uuid::nil(),
            Uuid::nil(),
            Uuid::nil(),
            "Publish the calculator project".to_string(),
            VibeRunConfig::default(),
        );
        assert_eq!(
            AgentLoop::local_forced_tool("http://127.0.0.1:8081/v1/chat/completions", &run),
            if cfg!(windows) {
                Some("publish/project")
            } else {
                None
            }
        );
    }

    #[test]
    fn local_windows_builds_exact_title_tool_arguments() {
        let run = VibeRun::new(
            Uuid::nil(),
            Uuid::nil(),
            Uuid::nil(),
            "In project calculator-custom: Change the calculator page title to XCalculator."
                .to_string(),
            VibeRunConfig::default(),
        );
        let response = AgentLoop::local_deterministic_tool_response(
            "http://localhost:8081/v1/chat/completions",
            &run,
        );
        if cfg!(windows) {
            let parsed: serde_json::Value =
                serde_json::from_str(response.as_deref().unwrap_or_default()).unwrap_or_default();
            assert_eq!(parsed["tool_calls"][0]["arguments"]["title"], "XCalculator");
        } else {
            assert!(response.is_none());
        }
    }

    #[test]
    fn last_verdict_uses_final_token() {
        // Historical FAILED mention must not override a trailing VERIFIED.
        assert_eq!(
            AgentLoop::last_verdict("The first test FAILED but was corrected. VERIFIED"),
            Some(true)
        );
        assert_eq!(AgentLoop::last_verdict("Looks good. VERIFIED"), Some(true));
        assert_eq!(
            AgentLoop::last_verdict("FAILED: server.js still missing"),
            Some(false)
        );
        assert_eq!(
            AgentLoop::last_verdict("Everything passed: VERIFIED. Wait, no — FAILED"),
            Some(false)
        );
        assert_eq!(AgentLoop::last_verdict("No explicit verdict here"), None);
        assert_eq!(AgentLoop::last_verdict(""), None);
    }

    #[test]
    fn extract_json_object_handles_nested_and_strings() {
        let input = r#"prefix {"a": {"b": [1, 2]}, "c": "x{y}z"} suffix"#;
        let extracted = extract_json_object(input).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&extracted).unwrap();
        assert_eq!(parsed["a"]["b"][1], 2);
        assert_eq!(parsed["c"], "x{y}z");
    }

    #[test]
    fn extract_json_object_returns_none_for_no_object() {
        assert!(extract_json_object("no braces here").is_none());
        assert!(extract_json_object("").is_none());
        assert!(extract_json_object("unbalanced {").is_none());
    }

    #[test]
    fn looks_like_tool_intent_detects_json_tool_phrases() {
        assert!(looks_like_tool_intent(r#"{"tool_calls": []}"#));
        assert!(looks_like_tool_intent(r#"use tool_name read_file"#));
        assert!(!looks_like_tool_intent("no tools needed, task done"));
        assert!(!looks_like_tool_intent(""));
    }

    #[test]
    fn truncate_keeps_short_and_marks_long() {
        assert_eq!(truncate("short", 100), "short");
        let text = "a".repeat(5000);
        let out = truncate(&text, 4000);
        assert!(out.len() <= 4016);
        assert!(out.ends_with("(truncated)"));
        assert!(out.contains('…'));
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        let text = "ç".repeat(3000);
        let out = truncate(&text, 100);
        assert!(out.is_char_boundary(out.len()));
    }

    #[test]
    fn native_tool_calls_parse_openai_format() {
        let payload = serde_json::json!({
            "choices": [{"message": {"tool_calls": [
                {"id": "call_1", "type": "function",
                 "function": {"name": "read_file", "arguments": "{\"path\": \"a.txt\"}"}}
            ]}}]
        });
        let calls = native_tool_calls_from_value(&payload).expect("native calls present");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "read_file");
        assert_eq!(calls[0].arguments["path"], "a.txt");
        assert!(native_tool_calls_from_value(
            &serde_json::json!({"choices": [{"message": {"content": "hi"}}]})
        )
        .is_none());
    }

    #[test]
    fn content_from_payload_errors_on_truncated_tool_arguments() {
        // A stream cut off mid-argument leaves invalid JSON; it must be
        // reported (so call_llm retries non-streaming) instead of silently
        // substituting `{}` and writing an empty/corrupt file.
        let truncated = serde_json::json!({
            "choices": [{"message": {"tool_calls": [
                {"id": "call_1", "type": "function",
                 "function": {"name": "file/write", "arguments": "{\"path\": \"server.js\", \"content\": \"const app = req"}}
            ]}}]
        });
        let err = AgentLoop::content_from_payload(&truncated).expect_err("must fail on truncation");
        assert!(
            err.contains("truncated"),
            "error should flag truncation: {err}"
        );

        // A complete call with valid JSON arguments parses normally.
        let valid = serde_json::json!({
            "choices": [{"message": {"tool_calls": [
                {"id": "call_1", "type": "function",
                 "function": {"name": "file/write", "arguments": "{\"path\": \"server.js\", \"content\": \"ok\"}"}}
            ]}}]
        });
        let out = AgentLoop::content_from_payload(&valid).expect("valid call parses");
        assert!(out.contains("file/write") && out.contains("server.js"));
    }

    #[test]
    fn canonical_json_envelope_parses_like_loop_output() {
        let calls = vec![ExtractedToolCall {
            tool_name: "file/read".to_string(),
            arguments: serde_json::json!({"path": "a.txt"}),
        }];
        let json = canonical_tool_calls_json(&calls);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("envelope is json");
        assert_eq!(parsed["tool_calls"][0]["tool_name"], "file/read");
        assert_eq!(parsed["tool_calls"][0]["arguments"]["path"], "a.txt");
    }

    #[test]
    fn parse_sse_accumulates_content_and_tool_call_deltas() {
        let lines = [
            format!("data: {}", json!({"choices": [{"delta": {"role": "assistant"}}]})),
            format!("data: {}", json!({"choices": [{"delta": {"content": "Hel"}}]})),
            format!("data: {}", json!({"choices": [{"delta": {"content": "lo"}}]})),
            format!("data: {}", json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "call_1", "function": {"name": "read_fi", "arguments": "{\"pa"}}]}}]})),
            format!("data: {}", json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "function": {"name": "le", "arguments": "th\": \"a.txt\"}"}}]}}]})),
        ]
        .join("\n")
            + "\ndata: [DONE]";
        let payload = parse_sse(&lines).expect("sse parsed");
        assert_eq!(payload["choices"][0]["message"]["content"], "Hello");
        assert_eq!(
            payload["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "read_file"
        );
        assert_eq!(
            payload["choices"][0]["message"]["tool_calls"][0]["function"]["arguments"],
            "{\"path\": \"a.txt\"}"
        );
    }

    #[test]
    fn parse_sse_rejects_non_sse_body() {
        assert!(parse_sse("{\"choices\":[]}").is_none());
    }

    #[test]
    fn parse_tool_calls_from_llm_response() {
        let agent = AgentLoop::new(
            Arc::new(VibePromptManager::new()),
            Arc::new(VibeToolExecutor::new(Arc::new(
                crate::tool_executor::ToolRegistry::new(),
            ))),
            Arc::new(VibeTelemetry::new()),
            Arc::new(MockState::new()),
        );
        let response = r#"{"tool_calls": [{"tool_name": "file/read", "arguments": {"path": "a.txt"}}, {"tool_name": "web/search", "arguments": {}}]}"#;
        let calls = agent.parse_tool_calls(response);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].tool_name, "file/read");
        assert_eq!(calls[0].arguments["path"], "a.txt");
        assert_eq!(calls[1].tool_name, "web/search");
        assert!(agent.parse_tool_calls("no calls").is_empty());
    }

    #[test]
    fn project_scope_overrides_model_workspace_arguments() {
        let config = crate::types::VibeRunConfig {
            project_id: Some("project-id".to_string()),
            project_name: Some("calculator-e2e".to_string()),
            ..crate::types::VibeRunConfig::default()
        };
        let run = VibeRun::new(
            Uuid::nil(),
            Uuid::nil(),
            Uuid::nil(),
            "edit".to_string(),
            config,
        );
        let extracted = ExtractedToolCall {
            tool_name: "file/write".to_string(),
            arguments: serde_json::json!({"project": "wrong-project", "path": "index.js"}),
        };
        let args = project_scoped_arguments(&run, &extracted);
        assert_eq!(args["project"], "calculator-e2e");
        assert_eq!(args["path"], "index.js");

        let publish = ExtractedToolCall {
            tool_name: "publish/project".to_string(),
            arguments: serde_json::json!({"project_id": "wrong-id"}),
        };
        assert_eq!(
            project_scoped_arguments(&run, &publish)["project_id"],
            "project-id"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn ambiguous_windows_edit_path_prefers_main_source_file() {
        let entries = vec![
            "README.md".to_string(),
            "calc.js".to_string(),
            "index.js".to_string(),
            "package.json".to_string(),
            "test.js".to_string(),
        ];
        assert!(path_requires_repair("."));
        assert!(path_requires_repair("/"));
        assert_eq!(
            preferred_source_path(&entries, "Change the calculator button background to blue"),
            Some("index.js".to_string())
        );
    }

    struct MockState {
        runs: Arc<RwLock<std::collections::HashMap<Uuid, VibeRun>>>,
    }

    impl MockState {
        fn new() -> Self {
            Self {
                runs: Arc::new(RwLock::new(std::collections::HashMap::new())),
            }
        }
    }

    impl VibeState for MockState {
        fn db_pool(&self) -> &crate::types::DbPool {
            unreachable!("db_pool not exercised in parse tests")
        }
        fn broadcast_progress(&self, _event: VibeProgressEvent) {}
        fn progress_sender(&self) -> Option<&tokio::sync::broadcast::Sender<VibeProgressEvent>> {
            None
        }
        fn active_runs(&self) -> &Arc<RwLock<std::collections::HashMap<Uuid, VibeRun>>> {
            &self.runs
        }
        fn run_signal_sender(
            &self,
        ) -> Option<&tokio::sync::broadcast::Sender<crate::types::VibeRunSignal>> {
            None
        }
        fn llm_config(&self, _bot_id: &uuid::Uuid) -> Option<crate::types::LlmConfig> {
            None
        }
    }

