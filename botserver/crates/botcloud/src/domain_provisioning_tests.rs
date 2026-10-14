//! Unit tests for `super` (domain_provisioning.rs).
//!
//! Split out of `domain_provisioning.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

fn sku() -> DomainSku {
    sku_for("domain-com").expect("sku")
}

#[test]
fn every_catalogued_domain_sku_maps_to_a_tld() {
    for (id, tld) in [("domain-com", "com"), ("domain-io", "io"), ("domain-ai", "ai")] {
        assert_eq!(sku_for(id).expect("sku").tld, tld);
    }
    assert!(sku_for("vps-small").is_none(), "a VPS is not a domain");
    assert!(sku_for("nonsense").is_none());
}

#[test]
fn no_domain_sku_depends_on_a_single_registrar() {
    for id in ["domain-com", "domain-io", "domain-ai"] {
        let sku = sku_for(id).expect("sku");
        assert!(sku.candidates.len() >= 2, "{id} has no fallback registrar");
    }
}

#[test]
fn every_candidate_can_actually_sell_the_tld() {
    for id in ["domain-com", "domain-io", "domain-ai"] {
        let sku = sku_for(id).expect("sku");
        for name in sku.candidates {
            let info = registry::registrar_info_for_name(name);
            let Some(info) = info else {
                assert_eq!(*name, "namecheap", "{id} lists an unbuildable registrar");
                continue;
            };
            assert!(
                info.supported_tlds.iter().any(|t| t == sku.tld),
                "{id} lists {name}, which does not sell .{}",
                sku.tld
            );
        }
    }
}

#[test]
fn a_provisioned_domain_name_is_derived_from_the_tenant() {
    let org = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").expect("uuid");
    let other = Uuid::parse_str("6ba7b811-9dad-11d1-80b4-00c04fd430c8").expect("uuid");
    let first = domain_for(org, "com");
    assert!(first.ends_with(".com"), "{first}");
    assert_ne!(first, domain_for(other, "com"), "two tenants must not collide");
    assert_eq!(first, domain_for(org, "com"), "the name must be stable");
}

#[test]
fn the_hostname_label_never_leaks_the_tenant_id() {
    let org = Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").expect("uuid");
    let host = domain_for(org, "com");
    assert!(
        !host.contains(&org.simple().to_string()),
        "{host} exposes the organization id"
    );
    let label = host.trim_start_matches("gb-").split('.').next().unwrap_or_default();
    assert!(label.len() == 10, "expected a fixed-width label, got {label}");
    assert!(label.chars().all(|c| c.is_ascii_alphanumeric()));
}

#[test]
fn base36_is_fixed_width_and_zero_padded() {
    assert_eq!(to_base36(0, 4), "0000");
    assert_eq!(to_base36(35, 4), "000z");
    assert_eq!(to_base36(36, 4), "0010");
}

#[test]
fn per_registrar_credentials_are_read_from_config() {
    let config = serde_json::json!({
        "registrar_keys": {
            "Porkbun": { "api_key": "pk1", "api_secret": "ps1" },
            "Dynadot": { "api_key": "", "api_secret": "" },
        },
        "domain_registrar": " DYNADOT ",
        "domain_app_ip": "203.0.113.7",
        "domain_dmarc": "v=DMARC1; p=none",
    });
    let settings = OrgDomainSettings::from_config(Some(&config));
    let porkbun = settings.key_for("porkbun").expect("credential");
    assert_eq!(porkbun.api_key, "pk1");
    assert_eq!(porkbun.api_secret, "ps1");
    assert!(settings.key_for("dynadot").is_none(), "an empty entry is not a credential");
    assert_eq!(settings.preferred_registrar.as_deref(), Some("dynadot"));
    assert_eq!(settings.app_ip, "203.0.113.7");
    assert_eq!(settings.dmarc_record, "v=DMARC1; p=none");
}

#[test]
fn a_configured_ip_reaches_the_onboarding_records() {
    let config = serde_json::json!({ "domain_app_ip": "198.51.100.4" });
    let settings = OrgDomainSettings::from_config(Some(&config));
    let records = zone::onboarding_records(
        &settings.app_ip,
        &settings.app_host,
        &settings.dmarc_record,
    );
    let apex = records.iter().find(|r| r.record_type == botdomains::RecordType::A).expect("apex");
    assert_eq!(apex.value, "198.51.100.4");
}

#[test]
fn an_absent_config_still_produces_a_usable_default_policy() {
    let settings = OrgDomainSettings::from_config(None);
    assert!(!settings.dmarc_record.is_empty());
    assert!(settings.dmarc_record.contains("p=reject"), "the default must be enforcing");
    assert!(!settings.app_ip.is_empty());
}

#[test]
fn a_preference_moves_the_registrar_to_the_front() {
    let config = serde_json::json!({ "domain_registrar": "dynadot" });
    let settings = OrgDomainSettings::from_config(Some(&config));
    let ordered = settings.order_candidates(&["porkbun", "dynadot", "cloudflare"]);
    assert_eq!(ordered, vec!["dynadot", "porkbun", "cloudflare"]);
}

#[test]
fn the_missing_credential_report_names_what_is_absent() {
    let settings = OrgDomainSettings::default();
    let report = missing_credential_report(&settings, &sku());
    assert!(report.contains("porkbun"), "{report}");
    assert!(report.contains("missing"), "{report}");
}

#[test]
fn the_report_is_empty_of_gaps_once_a_credential_exists() {
    let config = serde_json::json!({ "registrar_keys": { "porkbun": { "api_key": "k", "api_secret": "s" } } });
    let settings = OrgDomainSettings::from_config(Some(&config));
    let report = missing_credential_report(&settings, &sku());
    assert!(report.contains("missing: cloudflare"), "{report}");
}

#[tokio::test]
async fn a_chain_with_no_credentials_reports_that_not_a_silent_success() {
    let settings = OrgDomainSettings::default();
    let err = register_domain(&sku(), "gb-1.com", &settings)
        .await
        .expect_err("no credential means no registration");
    assert!(err.contains("no credential"), "{err}");
}

#[tokio::test]
async fn a_credential_for_a_registrar_that_cannot_sell_the_tld_is_skipped() {
    // Porkbun has a credential here but does not sell `.museum`, so the
    // attempt must be refused before any purchase rather than after one.
    let config = serde_json::json!({
        "registrar_keys": { "porkbun": { "api_key": "k", "api_secret": "s" } },
    });
    let settings = OrgDomainSettings::from_config(Some(&config));
    let exotic = DomainSku {
        tld: "museum",
        yearly_cents: 5_000,
        candidates: &["porkbun"],
    };
    let err = register_domain(&exotic, "gb-test.museum", &settings)
        .await
        .expect_err("porkbun does not sell .museum");
    assert!(err.contains("does not sell"), "{err}");
}

#[tokio::test]
async fn a_preference_outside_the_chain_is_ignored_rather_than_failing() {
    let config = serde_json::json!({
        "registrar_keys": { "desec": { "api_key": "k", "api_secret": "s" } },
        "domain_registrar": "desec",
    });
    let settings = OrgDomainSettings::from_config(Some(&config));
    let err = register_domain(&sku(), "gb-test.com", &settings)
        .await
        .expect_err("deSEC is not in the chain");
    assert!(err.contains("no credential"), "{err}");
}
