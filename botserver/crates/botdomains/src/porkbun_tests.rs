//! Unit tests for `super` (porkbun.rs).
//!
//! Split out of `porkbun.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

use crate::Registrar;

#[test]
fn a_fully_qualified_domain_splits_into_label_and_tld() {
    assert_eq!(
        split_domain("acme.com").expect("split"),
        ("acme".to_string(), "com".to_string())
    );
    assert_eq!(
        split_domain("ACME.IO.").expect("split"),
        ("acme".to_string(), "io".to_string())
    );
    assert_eq!(
        split_domain("a.b.example.com").expect("split"),
        ("a".to_string(), "b.example.com".to_string())
    );
}

#[test]
fn an_incomplete_domain_is_refused_before_any_request() {
    assert!(matches!(split_domain("acme"), Err(RegistrarError::Rejected(_, _))));
    assert!(matches!(split_domain(".com"), Err(RegistrarError::Rejected(_, _))));
    assert!(matches!(split_domain("acme."), Err(RegistrarError::Rejected(_, _))));
}

#[test]
fn application_level_errors_map_to_the_narrow_variant() {
    assert!(matches!(
        map_error("k", "Invalid API key."),
        RegistrarError::Credentials(_)
    ));
    assert!(matches!(
        map_error("k", "Domain is not available."),
        RegistrarError::Unavailable(_, _)
    ));
    assert!(matches!(
        map_error("k", "Rate limit exceeded."),
        RegistrarError::RateLimited(_, 60)
    ));
    assert!(matches!(
        map_error("k", "Something else went wrong."),
        RegistrarError::Rejected(_, _)
    ));
}

#[test]
fn the_adapter_reports_per_record_dns_and_no_zone_replace() {
    let info = Porkbun::new().info();
    assert!(info.manages_dns);
    assert!(!info.replaces_whole_zone, "Porkbun edits one record at a time");
    assert!(info.supported_tlds.contains(&"com".to_string()));
    assert!(info.supported_tlds.contains(&"ai".to_string()));
}

#[test]
fn every_catalogue_domain_tld_is_registerable_here() {
    let info = Porkbun::new().info();
    for tld in ["com", "io", "ai"] {
        assert!(info.supported_tlds.iter().any(|t| t == tld), "{tld} missing");
    }
}

#[test]
fn record_fields_carry_every_value_porkbun_needs() {
    let record = DnsRecord::new("www", RecordType::Cname, "target.example.net").with_ttl(60);
    let fields = record_fields("acme.com", &record);
    let get = |key: &str| {
        fields.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    };
    assert_eq!(get("domain").as_deref(), Some("acme.com"));
    assert_eq!(get("name").as_deref(), Some("www"));
    assert_eq!(get("type").as_deref(), Some("CNAME"));
    assert_eq!(get("content").as_deref(), Some("target.example.net"));
    assert_eq!(get("ttl").as_deref(), Some("60"));
}

#[tokio::test]
async fn tld_prices_are_flat_so_renewal_cannot_cliff() {
    let prices = Porkbun::new().tld_list(&Credential::default()).await.expect("prices");
    assert!(!prices.is_empty());
    for price in &prices {
        assert_eq!(price.registration_cents, price.renewal_cents, "{}", price.tld);
    }
}
