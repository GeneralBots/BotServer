use std::collections::HashSet;

use serde_json::Value;

use super::types::{AuthMethod, InputType, Risk, Status};
use super::{llm_actions, provider_by_id, search};

#[test]
fn provider_ids_and_names_are_unique() {
    let catalog = search(None, None, None);
    // 142 includes the ten keyless LLM inference seeds (e5b7d7202), Instagram
    // (d1e6a5688) and the Telegram/WhatsApp channel adapters (4de54895f), none
    // of which updated these snapshots when they landed.
    assert_eq!(catalog.provider_count, 142);
    assert_eq!(catalog.action_count, 766);
    assert_eq!(catalog.categories.len(), 8);
    assert_eq!(catalog.totals.providers, 142);
    assert_eq!(catalog.totals.actions, 766);
    // Slices 1-2 (#950): AWS, GitHub and Stripe have live adapters; all of
    // their catalog actions are implemented when the integrations feature
    // compiles the registry in (13 + 9 + 10), and none otherwise. Instagram
    // adds its three implementable Graph API actions on top of that.
    #[cfg(feature = "integrations")]
    assert_eq!(catalog.totals.implemented_actions, 35);
    #[cfg(not(feature = "integrations"))]
    assert_eq!(catalog.totals.implemented_actions, 0);
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for provider in &catalog.providers {
        assert!(ids.insert(provider.id));
        assert!(names.insert(provider.name));
    }
}

#[test]
fn github_and_stripe_flip_llm_available_without_status_drift() {
    let expectations = [("github", Status::Partial), ("stripe", Status::Built)];
    for (id, status) in expectations {
        let Some(provider) = provider_by_id(id) else {
            assert!(false, "{id} provider is missing");
            return;
        };
        assert!(
            provider.llm_available,
            "{id} must advertise LLM availability"
        );
        assert_eq!(provider.status, status, "{id} status must not drift");
    }
}

#[cfg(feature = "integrations")]
#[test]
fn github_and_stripe_registered_actions_match_catalog_exactly() {
    let cases = [
        (
            "github",
            botintegrations::providers::github::GITHUB_IMPLEMENTED_ACTIONS,
        ),
        (
            "stripe",
            botintegrations::providers::stripe::STRIPE_IMPLEMENTED_ACTIONS,
        ),
    ];
    for (id, registered) in cases {
        let Some(provider) = provider_by_id(id) else {
            assert!(false, "{id} provider is missing");
            return;
        };
        let registered_names: HashSet<String> =
            registered.iter().map(|key| format!("{id}.{key}")).collect();
        let registered_refs: HashSet<&str> = registered_names.iter().map(String::as_str).collect();
        let implemented: HashSet<&str> = provider
            .actions
            .iter()
            .filter(|action| action.implemented)
            .map(|action| action.name.as_str())
            .collect();
        assert_eq!(
            implemented, registered_refs,
            "{id} catalog profile and adapter registry diverge"
        );
        assert_eq!(
            provider.actions.len(),
            registered.len(),
            "{id} must not carry catalog-only actions without an adapter"
        );
    }
}

#[cfg(not(feature = "integrations"))]
#[test]
fn github_and_stripe_advertise_no_implemented_actions_without_the_registry() {
    for id in ["github", "stripe"] {
        let Some(provider) = provider_by_id(id) else {
            assert!(false, "{id} provider is missing");
            return;
        };
        assert!(!provider.llm_available);
        assert!(provider.actions.iter().all(|action| !action.implemented));
    }
}

#[test]
fn expanded_action_names_are_unique() {
    let catalog = search(None, None, None);
    let mut names = HashSet::new();
    for provider in &catalog.providers {
        let mut provider_names = HashSet::new();
        for action in &provider.actions {
            assert!(provider_names.insert(action.name.as_str()));
            assert!(names.insert(action.name.as_str()));
        }
    }
}

#[test]
fn aws_auth_and_actions_are_explicit_and_guarded() {
    let Some(aws) = provider_by_id("aws") else {
        assert!(false, "AWS provider is missing");
        return;
    };
    assert_eq!(aws.auth.method, AuthMethod::AccessKey);
    assert!(aws.official_docs.is_some());
    assert!(aws.llm_available);
    // Slice 1 (#950): the implemented flags mirror
    // botintegrations::providers::registry exactly for this build.
    let implemented_count = aws
        .actions
        .iter()
        .filter(|action| action.implemented)
        .count();
    #[cfg(feature = "integrations")]
    assert_eq!(implemented_count, 13);
    #[cfg(not(feature = "integrations"))]
    assert_eq!(implemented_count, 0);

    for key in ["access_key_id", "secret_access_key"] {
        let field = aws.auth.fields.iter().find(|field| field.key == key);
        assert!(field.is_some_and(|value| {
            value.secret && value.required && value.input_type == InputType::Password
        }));
    }
    for key in ["session_token", "region"] {
        let field = aws.auth.fields.iter().find(|field| field.key == key);
        assert!(field.is_some_and(|value| !value.required));
    }
    assert!(aws.auth.instructions.contains("workload identity"));
    assert!(aws.auth.instructions.contains("STS role"));
    assert!(aws.auth.instructions.contains("minimum custom IAM policy"));
    assert!(aws
        .auth
        .instructions
        .contains("never use root or AdministratorAccess"));
    assert!(aws.auth.instructions.contains("never enter LLM context"));

    let expected = [
        "aws.sts.caller_identity.get",
        "aws.s3.objects.list",
        "aws.s3.objects.search",
        "aws.s3.objects.get",
        "aws.s3.objects.put",
        "aws.s3.objects.delete",
        "aws.ec2.instances.describe",
        "aws.ec2.instances.start",
        "aws.ec2.instances.stop",
        "aws.cloudwatch.metrics.query",
        "aws.cloudwatch.logs.search",
        "aws.lambda.functions.list",
        "aws.lambda.functions.invoke",
    ];
    for name in expected {
        assert!(aws.actions.iter().any(|action| action.name == name));
    }
    for verb in ["put", "delete", "start", "stop", "invoke"] {
        let matching: Vec<_> = aws
            .actions
            .iter()
            .filter(|action| action.verb == verb)
            .collect();
        assert!(!matching.is_empty());
        assert!(matching
            .iter()
            .all(|action| action.requires_approval && action.risk != Risk::Low));
    }
}

#[test]
fn llm_action_manifest_omits_secret_auth_metadata() {
    let Some(aws) = provider_by_id("aws") else {
        assert!(false, "AWS provider is missing");
        return;
    };
    let Some(actions) = llm_actions("aws") else {
        assert!(false, "AWS LLM action manifest is missing");
        return;
    };
    let value = match serde_json::to_value(actions) {
        Ok(value) => value,
        Err(error) => {
            assert!(false, "LLM action manifest serialization failed: {error}");
            return;
        }
    };
    let encoded = value.to_string();
    for field in aws.auth.fields.iter().filter(|field| field.secret) {
        assert!(!encoded.contains(field.key));
        assert!(!encoded.contains(field.label));
    }
    assert!(!encoded.contains("instructions"));
    assert!(!encoded.contains("least_privilege"));
    assert!(!encoded.contains("auth"));

    let Value::Array(items) = value else {
        assert!(false, "LLM action manifest must be an array");
        return;
    };
    let allowed: HashSet<&str> = [
        "name",
        "summary",
        "params",
        "risk",
        "requires_approval",
        "implemented",
    ]
    .into_iter()
    .collect();
    for item in items {
        let Value::Object(fields) = item else {
            assert!(false, "LLM action entries must be objects");
            continue;
        };
        assert!(fields.keys().all(|key| allowed.contains(key.as_str())));
    }
}

#[tokio::test]
#[ignore = "persistent browser contract server"]
async fn serve_catalog_for_browser() {
    use axum::{routing::get, Json, Router};
    use serde_json::json;

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:8080").await {
        Ok(listener) => listener,
        Err(error) => {
            assert!(false, "browser contract server bind failed: {error}");
            return;
        }
    };
    let router = super::register(Router::new())
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route(
            "/api/product",
            get(|| async { Json(json!({ "sidebar": false })) }),
        )
        .route(
            "/api/integrations/connectors",
            get(|| async { Json(json!({ "connectors": [] })) }),
        );
    let result = axum::serve(listener, router).await;
    assert!(result.is_ok(), "browser contract server failed: {result:?}");
}
