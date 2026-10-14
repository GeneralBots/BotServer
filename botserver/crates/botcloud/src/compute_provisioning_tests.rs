//! Unit tests for `super` (compute_provisioning.rs).
//!
//! Split out of `compute_provisioning.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

#[test]
fn every_catalogue_compute_sku_maps_to_a_spec() {
    for id in [
        "vps-small", "vps-medium", "vps-large", "vps-xl", "gpu-basic", "gpu-pro",
        "gpu-enterprise",
    ] {
        assert!(sku_for(id).is_some(), "{id} has no spec");
    }
    assert!(sku_for("domain-com").is_none(), "a domain is not a machine");
    assert!(sku_for("nonsense").is_none());
}

#[test]
fn no_sku_depends_on_a_single_provider() {
    for id in [
        "vps-small", "vps-medium", "vps-large", "vps-xl", "gpu-basic", "gpu-pro",
        "gpu-enterprise",
    ] {
        let sku = sku_for(id).expect("spec");
        assert!(
            sku.candidates.len() >= 2,
            "{id} would be unprovisionable when its only provider is out"
        );
    }
}

#[test]
fn a_gpu_sku_never_candidates_a_cpu_only_host() {
    for id in ["gpu-basic", "gpu-pro", "gpu-enterprise"] {
        let sku = sku_for(id).expect("spec");
        assert!(sku.machine.gpu_type.is_some(), "{id} carries no GPU");
        for name in sku.candidates {
            let info = registry::provider_info_for_name(name);
            let Some(info) = info else { continue };
            assert!(
                !info.available_gpus.is_empty(),
                "{id} lists {name}, which advertises no GPU line"
            );
        }
    }
}

#[test]
fn a_cpu_sku_never_candidates_a_gpu_only_host() {
    for id in ["vps-small", "vps-medium", "vps-large", "vps-xl"] {
        let sku = sku_for(id).expect("spec");
        assert!(sku.machine.gpu_type.is_none(), "{id} asks for a GPU");
    }
}

#[test]
fn per_provider_keys_are_read_from_config() {
    let config = serde_json::json!({
        "provider_keys": { "Hetzner": "hz-key", "ovh": "" },
        "compute_provider": " OVH ",
    });
    let settings = OrgComputeSettings::from_config(Some(&config));
    assert_eq!(settings.key_for("hetzner"), Some("hz-key"));
    assert_eq!(settings.key_for("ovh"), None, "an empty value is not a key");
    assert_eq!(settings.preferred_provider.as_deref(), Some("ovh"));
}

#[test]
fn the_legacy_single_key_still_resolves() {
    let config = serde_json::json!({ "provider_api_key": "legacy" });
    let settings = OrgComputeSettings::from_config(Some(&config));
    assert_eq!(settings.key_for("hetzner"), Some("legacy"));
    assert_eq!(settings.keys.len(), 0);
}

#[test]
fn a_named_key_beats_the_legacy_one() {
    let config = serde_json::json!({
        "provider_api_key": "legacy",
        "provider_keys": { "hetzner": "hz" },
    });
    let settings = OrgComputeSettings::from_config(Some(&config));
    assert_eq!(settings.key_for("hetzner"), Some("hz"));
    assert_eq!(settings.key_for("vultr"), Some("legacy"));
}

#[test]
fn missing_or_unparseable_config_is_not_an_error() {
    assert_eq!(OrgComputeSettings::from_config(None).keys.len(), 0);
    assert_eq!(OrgComputeSettings::from_config(Some(&Value::Null)).keys.len(), 0);
}

#[test]
fn a_preference_moves_the_provider_to_the_front() {
    let config = serde_json::json!({ "compute_provider": "vultr" });
    let settings = OrgComputeSettings::from_config(Some(&config));
    let ordered = settings.order_candidates(&["hetzner", "vultr", "contabo"]);
    assert_eq!(ordered, vec!["vultr", "hetzner", "contabo"]);
}

#[test]
fn a_preference_outside_the_chain_changes_nothing() {
    let config = serde_json::json!({ "compute_provider": "oracle" });
    let settings = OrgComputeSettings::from_config(Some(&config));
    let candidates = ["hetzner", "vultr"];
    assert_eq!(settings.order_candidates(&candidates), vec!["hetzner", "vultr"]);
}

#[test]
fn cost_within_tolerance_is_not_reported() {
    // Catalogue $19.99/month is ~2.7c/hour; $0.028 is inside 5%.
    assert!(reconcile_cost(0.028, 19.99).is_none());
}

#[test]
fn cost_above_tolerance_is_reported_as_a_percentage() {
    let variance = reconcile_cost(0.10, 19.99).expect("variance reported");
    assert!(variance > COST_TOLERANCE_PCT, "{variance}");
}

#[test]
fn a_provider_that_reports_no_price_is_not_treated_as_free() {
    assert!(reconcile_cost(0.0, 19.99).is_none());
    assert!(reconcile_cost(5.0, 0.0).is_none());
}

#[test]
fn the_stored_config_carries_the_variance_when_present() {
    let outcome = ProvisionOutcome {
        result: ProvisionResult {
            provider: "hetzner".into(),
            instance_id: "42".into(),
            status: "running".into(),
            ip_address: Some("203.0.113.7".into()),
            region: "hel1".into(),
            spec: MachineSpec::default(),
            hourly_cost: 0.04,
        },
        provider_name: "hetzner".into(),
        cost_variance_pct: Some(42.0),
    };
    let config = success_config(&outcome);
    assert_eq!(config["provider"], "hetzner");
    assert_eq!(config["cost_variance_pct"], 42.0);

    let without = success_config(&ProvisionOutcome {
        result: outcome.result.clone(),
        provider_name: "hetzner".into(),
        cost_variance_pct: None,
    });
    assert!(without.get("cost_variance_pct").is_none());
}

#[tokio::test]
async fn a_chain_with_no_credentials_reports_that_not_a_silent_success() {
    let sku = sku_for("vps-small").expect("spec");
    let settings = OrgComputeSettings::default();
    let err = provision_with_fallback(&sku, "US", &settings)
        .await
        .expect_err("no credential means no provisioning");
    assert!(err.contains("no credential"), "{err}");
}

#[tokio::test]
async fn a_chain_of_uncompiled_providers_names_them() {
    let sku = SkuSpec {
        machine: MachineSpec::default(),
        candidates: &["not-a-provider", "also-missing"],
    };
    let config = serde_json::json!({ "provider_keys": { "not-a-provider": "k" } });
    let settings = OrgComputeSettings::from_config(Some(&config));
    let err = provision_with_fallback(&sku, "US", &settings)
        .await
        .expect_err("nothing is compiled");
    assert!(err.contains("not-a-provider"), "{err}");
}
