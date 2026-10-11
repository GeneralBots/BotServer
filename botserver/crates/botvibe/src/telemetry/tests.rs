//! `telemetry::tests` — split per #1443.

use super::*;



    fn event(run_id: Uuid, event_type: VibeTelemetryEventType, success: bool, latency: u64, cost: f64) -> VibeTelemetryEvent {
        VibeTelemetryEvent {
            event_id: Uuid::new_v4(),
            run_id,
            event_type,
            tool_name: None,
            use_case: VibeUseCase::SoftwareDevelopment,
            latency_ms: latency,
            tokens_used: Some(10),
            estimated_cost: cost,
            success,
            error: None,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn record_and_metrics_aggregate_correctly() {
        let telemetry = VibeTelemetry::new();
        let run_id = Uuid::new_v4();

        telemetry.record(event(run_id, VibeTelemetryEventType::RunStarted, true, 0, 0.0)).await;
        telemetry.record(event(run_id, VibeTelemetryEventType::ToolCallCompleted, true, 10, 0.5)).await;
        telemetry.record(event(run_id, VibeTelemetryEventType::ToolCallCompleted, true, 20, 0.5)).await;
        telemetry.record(event(run_id, VibeTelemetryEventType::ToolCallFailed, false, 5, 0.1)).await;

        let m = telemetry.get_run_metrics(run_id).await.unwrap();
        assert_eq!(m.total_tool_calls, 3);
        assert_eq!(m.successful_tool_calls, 2);
        assert_eq!(m.failed_tool_calls, 1);
        assert_eq!(m.total_tokens, 40);
        assert_eq!(m.total_cost, 1.1);
        assert!((m.success_rate - 2.0 / 3.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn run_end_keeps_metrics_available() {
        // #1268 — per-run metrics must stay queryable after the run finishes
        // (the frontend reads them exactly on completion), so a terminal run
        // still returns its summary instead of None.
        let telemetry = VibeTelemetry::new();
        let run_id = Uuid::new_v4();
        telemetry.record(event(run_id, VibeTelemetryEventType::RunStarted, true, 0, 0.0)).await;
        telemetry.record(event(run_id, VibeTelemetryEventType::ToolCallCompleted, true, 40, 1.0)).await;
        telemetry.record(event(run_id, VibeTelemetryEventType::RunCompleted, true, 100, 2.0)).await;
        let m = telemetry.get_run_metrics(run_id).await.expect("metrics survive completion");
        assert_eq!(m.total_tool_calls, 1);
        assert_eq!(m.successful_tool_calls, 1);
    }

    #[tokio::test]
    async fn global_metrics_tally_runs_and_tools() {
        let telemetry = VibeTelemetry::new();
        let run_id = Uuid::new_v4();
        telemetry.record(event(run_id, VibeTelemetryEventType::RunStarted, true, 0, 0.0)).await;
        telemetry.record(event(run_id, VibeTelemetryEventType::RunCompleted, true, 30, 1.0)).await;
        telemetry.record(event(Uuid::new_v4(), VibeTelemetryEventType::RunStarted, true, 0, 0.0)).await;
        telemetry.record(event(Uuid::new_v4(), VibeTelemetryEventType::RunFailed, false, 5, 0.2)).await;
        telemetry.record(event(Uuid::new_v4(), VibeTelemetryEventType::ToolCallCompleted, true, 4, 0.1)).await;

        let g = telemetry.get_global_metrics().await;
        assert_eq!(g.total_runs, 2);
        assert_eq!(g.completed_runs, 1);
        assert_eq!(g.failed_runs, 1);
        assert_eq!(g.total_tool_calls, 1);
        assert!((g.total_cost - 1.3).abs() < 1e-9);
        assert_eq!(g.by_use_case.len(), 1);
    }

    #[tokio::test]
    async fn empty_telemetry_has_no_run_metrics() {
        let telemetry = VibeTelemetry::new();
        assert!(telemetry.get_run_metrics(Uuid::new_v4()).await.is_none());
        let g = telemetry.get_global_metrics().await;
        assert_eq!(g.total_runs, 0);
        assert!(g.by_use_case.is_empty());
    }

    #[tokio::test]
    async fn events_for_run_are_newest_first_and_limited() {
        let telemetry = VibeTelemetry::new();
        let run_id = Uuid::new_v4();
        for _ in 0..5 {
            telemetry.record(event(run_id, VibeTelemetryEventType::ToolCallCompleted, true, 1, 0.0)).await;
        }
        let events = telemetry.get_events_for_run(run_id, 2).await;
        assert_eq!(events.len(), 2);
        let all = telemetry.get_events_for_run(run_id, 100).await;
        assert_eq!(all.len(), 5);
    }

    #[tokio::test]
    async fn record_helpers_create_expected_event_types() {
        let telemetry = VibeTelemetry::new();
        let mut run = crate::types::VibeRun::new(Uuid::nil(), Uuid::nil(), Uuid::nil(), "i".into(), crate::types::VibeRunConfig::default());
        let run_id = run.run_id;

        telemetry.record_run_start(&run).await;
        run.transition(crate::types::VibeRunState::Completed);
        telemetry.record_run_completion(&run, 42, Some(100), 0.7).await;
        telemetry.record_tool_call(ToolCallRecord {
            run_id,
            use_case: VibeUseCase::SoftwareDevelopment,
            tool_name: "web/search".into(),
            latency_ms: 9,
            tokens: Some(20),
            cost: 0.01,
            success: true,
            error: None,
            metadata: std::collections::HashMap::new(),
        }).await;

        let events = telemetry.get_events_for_run(run_id, 100).await;
        assert_eq!(events.len(), 3);
        assert!(events.iter().any(|e| e.event_type == VibeTelemetryEventType::RunStarted));
        assert!(events.iter().any(|e| e.event_type == VibeTelemetryEventType::RunCompleted));
        assert!(events.iter().any(|e| e.event_type == VibeTelemetryEventType::ToolCallCompleted && e.tool_name.as_deref() == Some("web/search")));
    }

