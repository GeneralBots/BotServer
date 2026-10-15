use crate::api::VibeApiInner;
use crate::pipeline::RunPipeline;
use crate::types::{VibeRun, VibeTelemetryEvent, VibeTelemetryEventType};
use axum::{
    extract::{Extension, Path},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct GroundingSource {
    pub kind: String,
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GroundingResponse {
    pub success: bool,
    pub sources: Vec<GroundingSource>,
    pub error: Option<String>,
}

const MAX_SOURCES: usize = 40;

pub(crate) async fn get_run_grounding(
    Extension(api): Extension<Arc<VibeApiInner>>,
    Path(run_id): Path<Uuid>,
) -> impl IntoResponse {
    let sources = api.grounding_for(run_id).await;
    Json(GroundingResponse {
        success: true,
        sources,
        error: None,
    })
}

pub fn build_grounding(run: Option<&VibeRun>, events: &[VibeTelemetryEvent]) -> Vec<GroundingSource> {
    let mut sources: Vec<GroundingSource> = Vec::new();
    if let Some(run) = run {
        push_unique(
            &mut sources,
            GroundingSource {
                kind: "intent".to_string(),
                label: "intent".to_string(),
                detail: Some(run.intent.clone()),
            },
        );
        push_unique(
            &mut sources,
            GroundingSource {
                kind: "run".to_string(),
                label: format!("use_case/{}", run.use_case),
                detail: Some(format!("state: {}", run.state)),
            },
        );
        for stage in RunPipeline::for_use_case(run.use_case).stages {
            push_unique(
                &mut sources,
                GroundingSource {
                    kind: "stage".to_string(),
                    label: format!("stage/{}", stage.id),
                    detail: Some(stage.name.clone()),
                },
            );
        }
        for call in &run.tool_calls {
            push_tool_source(&mut sources, call.tool_name.clone(), &call.arguments);
        }
    }
    for event in events {
        if let Some(ref tool) = event.tool_name {
            let is_tool_event = matches!(
                event.event_type,
                VibeTelemetryEventType::ToolCallStarted
                    | VibeTelemetryEventType::ToolCallCompleted
                    | VibeTelemetryEventType::ToolCallFailed
            );
            if is_tool_event {
                push_tool_source(&mut sources, tool.clone(), &serde_json::json!({}));
            }
        }
        for (key, value) in &event.metadata {
            if key.contains("file") || key.contains("path") || key.contains("source") {
                push_unique(
                    &mut sources,
                    GroundingSource {
                        kind: "file".to_string(),
                        label: value.clone(),
                        detail: Some(key.clone()),
                    },
                );
            }
        }
        if sources.len() >= MAX_SOURCES {
            break;
        }
    }
    sources.truncate(MAX_SOURCES);
    sources
}

fn push_tool_source(sources: &mut Vec<GroundingSource>, tool_name: String, arguments: &serde_json::Value) {
    let label = format!("tool/{}", tool_name);
    let mut detail = None;
    if let Some(obj) = arguments.as_object() {
        for key in ["path", "file", "filename", "url"] {
            if let Some(serde_json::Value::String(value)) = obj.get(key) {
                if !value.is_empty() {
                    detail = Some(format!("{key}: {value}"));
                    break;
                }
            }
        }
    }
    push_unique(sources, GroundingSource {
        kind: "tool".to_string(),
        label,
        detail,
    });
}

pub fn sources_for_run(run: &VibeRun) -> Vec<String> {
    let mut refs: Vec<String> = Vec::new();
    push_ref(&mut refs, format!("intent: {}", run.intent));
    push_ref(&mut refs, format!("use_case: {}", run.use_case));
    for stage in RunPipeline::for_use_case(run.use_case).stages {
        push_ref(&mut refs, format!("pipeline stage: {}", stage.name));
    }
    for call in &run.tool_calls {
        push_ref(&mut refs, format!("tool invoked: {}", call.tool_name));
        for key in ["path", "file", "filename", "url"] {
            let in_args = call
                .arguments
                .as_object()
                .and_then(|obj| obj.get(key))
                .and_then(serde_json::Value::as_str)
                .filter(|v| !v.is_empty());
            if let Some(value) = in_args {
                push_ref(&mut refs, format!("file: {value}"));
                break;
            }
        }
        if let Some(result) = &call.result {
            let data = result.data.as_object();
            for key in ["path", "file", "filename", "url"] {
                let in_result = data
                    .and_then(|obj| obj.get(key))
                    .and_then(serde_json::Value::as_str)
                    .filter(|v| !v.is_empty());
                if let Some(value) = in_result {
                    push_ref(&mut refs, format!("file: {value}"));
                    break;
                }
            }
        }
    }
    refs
}

fn push_ref(refs: &mut Vec<String>, value: String) {
    if value.trim().is_empty() || refs.iter().any(|r| r == &value) {
        return;
    }
    refs.push(value);
}

fn push_unique(sources: &mut Vec<GroundingSource>, entry: GroundingSource) {
    let existing = sources
        .iter_mut()
        .find(|s| s.kind == entry.kind && s.label == entry.label);
    match existing {
        Some(found) => {
            if found.detail.is_none() {
                found.detail = entry.detail;
            }
        }
        None => sources.push(entry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{VibeRunConfig, VibeToolCall, VibeUseCase};
    use std::collections::HashMap;

    fn sample_run() -> VibeRun {
        let config = VibeRunConfig {
            use_case: VibeUseCase::CustomerSupport,
            ..Default::default()
        };
        VibeRun::new(Uuid::nil(), Uuid::nil(), Uuid::nil(), "help customer with refund".into(), config)
    }

    fn tool_event(tool: &str, metadata: HashMap<String, String>) -> VibeTelemetryEvent {
        VibeTelemetryEvent {
            event_id: Uuid::new_v4(),
            run_id: Uuid::nil(),
            event_type: VibeTelemetryEventType::ToolCallCompleted,
            tool_name: Some(tool.to_string()),
            use_case: VibeUseCase::SoftwareDevelopment,
            latency_ms: 5,
            tokens_used: Some(1),
            estimated_cost: 0.0,
            success: true,
            error: None,
            timestamp: chrono::Utc::now(),
            metadata,
        }
    }

    #[test]
    fn builds_intent_run_and_stage_sources() {
        let sources = build_grounding(Some(&sample_run()), &[]);
        assert!(sources.iter().any(|s| s.kind == "intent"));
        assert!(sources.iter().any(|s| s.kind == "run"));
        assert_eq!(sources.iter().filter(|s| s.kind == "stage").count(), 3);
    }

    #[test]
    fn dedupes_repeated_tool_sources() {
        let args = serde_json::json!({ "path": "/tmp/app/main.rs" });
        let mut sources = Vec::new();
        for _ in 0..3 {
            push_tool_source(&mut sources, "files/write".to_string(), &args);
        }
        assert_eq!(
            sources
                .iter()
                .filter(|s| s.kind == "tool" && s.label == "tool/files/write")
                .count(),
            1
        );
        assert_eq!(sources[0].detail.as_deref(), Some("path: /tmp/app/main.rs"));
    }

    #[test]
    fn telemetry_metadata_files_become_sources() {
        let mut meta = HashMap::new();
        meta.insert("file".to_string(), "/tmp/app/src/lib.rs".to_string());
        let events = vec![tool_event("files/write", meta)];
        let sources = build_grounding(None, &events);
        assert!(sources.iter().any(|s| s.kind == "file"));
    }

    #[test]
    fn no_run_still_reports_event_grounding() {
        let events = vec![tool_event("git/commit", HashMap::new())];
        let sources = build_grounding(None, &events);
        assert!(sources.iter().any(|s| s.label == "tool/git/commit"));
    }

    #[test]
    fn tool_calls_without_live_run_are_not_lost() {
        let mut run = sample_run();
        run.tool_calls = vec![VibeToolCall::new(
            Uuid::new_v4(),
            "web/search".to_string(),
            serde_json::json!({ "query": "refund policy" }),
            false,
        )];
        let sources = build_grounding(Some(&run), &[]);
        assert!(sources.iter().any(|s| s.label == "tool/web/search"));
    }

    #[test]
    fn truncates_at_max_sources() {
        let mut run = sample_run();
        for i in 0..50 {
            run.tool_calls.push(VibeToolCall::new(
                Uuid::new_v4(),
                format!("tool_{i}"),
                serde_json::json!({}),
                false,
            ));
        }
        let sources = build_grounding(Some(&run), &[]);
        assert!(sources.len() <= MAX_SOURCES);
    }

    #[test]
    fn sources_for_run_include_intent_stages_and_use_case() {
        let run = sample_run();
        let refs = sources_for_run(&run);
        assert!(refs.iter().any(|r| r.starts_with("intent:")));
        assert!(refs.iter().any(|r| r.starts_with("use_case: customer_support")));
        assert_eq!(
            refs.iter().filter(|r| r.starts_with("pipeline stage:")).count(),
            3
        );
    }

    #[test]
    fn sources_for_run_extract_file_refs_from_args_and_results() {
        let mut run = sample_run();
        let mut call = VibeToolCall::new(
            Uuid::new_v4(),
            "files/write".to_string(),
            serde_json::json!({ "path": "/tmp/app/main.rs" }),
            false,
        );
        call.result = Some(crate::types::VibeToolResult {
            success: true,
            data: serde_json::json!({ "path": "/tmp/app/main.rs" }),
            error: None,
            latency_ms: 3,
        });
        run.tool_calls.push(call);
        let refs = sources_for_run(&run);
        assert!(refs.iter().any(|r| r == "tool invoked: files/write"));
        assert_eq!(
            refs.iter().filter(|r| *r == "file: /tmp/app/main.rs").count(),
            1
        );
    }

    #[test]
    fn sources_for_run_dedupe_repeated_tools() {
        let mut run = sample_run();
        for _ in 0..3 {
            run.tool_calls.push(VibeToolCall::new(
                Uuid::new_v4(),
                "web/search".to_string(),
                serde_json::json!({}),
                false,
            ));
        }
        let refs = sources_for_run(&run);
        assert_eq!(
            refs.iter().filter(|r| *r == "tool invoked: web/search").count(),
            1
        );
    }
}