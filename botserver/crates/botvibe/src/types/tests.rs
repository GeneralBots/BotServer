//! `types::tests` — split per #1443.

use super::*;


    use serde_json::json;

    #[test]
    fn run_state_display() {
        assert_eq!(VibeRunState::Pending.to_string(), "pending");
        assert_eq!(VibeRunState::Running.to_string(), "running");
        assert_eq!(VibeRunState::AwaitingApproval.to_string(), "awaiting_approval");
        assert_eq!(VibeRunState::Completed.to_string(), "completed");
        assert_eq!(VibeRunState::Failed.to_string(), "failed");
        assert_eq!(VibeRunState::Cancelled.to_string(), "cancelled");
    }

    #[test]
    fn terminal_state_is_absorbing() {
        let mut run = VibeRun::new(Uuid::new_v4(), Uuid::nil(), Uuid::nil(), "i".into(), VibeRunConfig::default());
        run.transition(VibeRunState::Completed);
        assert_eq!(run.state, VibeRunState::Completed);
        let completed_at = run.completed_at;

        // A late approval/cancel or a lingering loop must not regress the run.
        run.transition(VibeRunState::Running);
        assert_eq!(run.state, VibeRunState::Completed);
        assert_eq!(run.completed_at, completed_at);
    }

    #[test]
    fn run_state_is_terminal() {
        assert!(!VibeRunState::Pending.is_terminal());
        assert!(!VibeRunState::Running.is_terminal());
        assert!(!VibeRunState::AwaitingApproval.is_terminal());
        assert!(VibeRunState::Completed.is_terminal());
        assert!(VibeRunState::Failed.is_terminal());
        assert!(VibeRunState::Cancelled.is_terminal());
    }

    #[test]
    fn use_case_display() {
        assert_eq!(VibeUseCase::SoftwareDevelopment.to_string(), "software_development");
        assert_eq!(VibeUseCase::CustomerSupport.to_string(), "customer_support");
        assert_eq!(VibeUseCase::FinancialAnalysis.to_string(), "financial_analysis");
    }

    #[test]
    fn run_config_defaults() {
        let cfg = VibeRunConfig::default();
        assert_eq!(cfg.use_case, VibeUseCase::SoftwareDevelopment);
        assert_eq!(cfg.lang, "en");
        assert!(!cfg.auto_approve);
        assert_eq!(cfg.max_tool_calls, 50);
        assert_eq!(cfg.timeout_seconds, 600);
        assert_eq!(cfg.model, None);
        assert_eq!(cfg.llm_key, None);
        assert_eq!(cfg.llm_url, None);
        assert_eq!(cfg.budget_cents, 0);
    }

    #[test]
    fn run_new_and_terminal_transitions_set_completed_at() {
        let run_id = Uuid::new_v4();
        let mut run = VibeRun::new(run_id, Uuid::nil(), Uuid::nil(), "intent".into(), VibeRunConfig::default());
        assert_eq!(run.state, VibeRunState::Pending);
        assert_eq!(run.bot_id, run_id);
        assert!(run.completed_at.is_none());

        run.transition(VibeRunState::Running);
        assert!(run.completed_at.is_none());

        run.transition(VibeRunState::Completed);
        assert_eq!(run.state, VibeRunState::Completed);
        assert!(run.completed_at.is_some());

        let mut failed = VibeRun::new(Uuid::new_v4(), Uuid::nil(), Uuid::nil(), "i".into(), VibeRunConfig::default());
        failed.transition(VibeRunState::Failed);
        assert!(failed.completed_at.is_some());

        let mut cancelled = VibeRun::new(Uuid::new_v4(), Uuid::nil(), Uuid::nil(), "i".into(), VibeRunConfig::default());
        cancelled.transition(VibeRunState::Cancelled);
        assert!(cancelled.completed_at.is_some());
    }

    #[test]
    fn context_tracks_messages_and_defaults() {
        let ctx = VibeContext::new(Uuid::nil());
        assert!(ctx.system_prompt.is_empty());
        assert!(ctx.conversation_history.is_empty());

        let mut ctx = ctx;
        ctx.add_user_message("hi".into());
        ctx.add_assistant_message("hello".into());
        assert_eq!(ctx.conversation_history.len(), 2);
        assert_eq!(ctx.conversation_history[0].role, "user");
        assert_eq!(ctx.conversation_history[1].role, "assistant");
    }

    #[test]
    fn tool_call_requires_approval_defaults_to_false() {
        let call = VibeToolCall::new(Uuid::new_v4(), "git/push".into(), json!({}), true);
        assert_eq!(call.tool_name, "git/push");
        assert!(call.requires_approval);
        assert!(!call.approved);
        assert!(call.result.is_none());
    }

    #[test]
    fn progress_event_started_shape() {
        let ev = VibeProgressEvent::started("run-1", "starting", 5);
        assert_eq!(ev.event_type, "vibe_started");
        assert_eq!(ev.run_id, "run-1");
        assert_eq!(ev.total_steps, 5);
        assert_eq!(ev.progress, 0);
        assert!(!ev.timestamp.is_empty());
    }

