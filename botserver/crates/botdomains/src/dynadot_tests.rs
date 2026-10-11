//! Unit tests for `super` (dynadot.rs).
//!
//! Split out of `dynadot.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

use crate::Registrar;

#[test]
fn an_incomplete_domain_is_refused() {
    assert!(split_domain("acme").is_err());
    assert!(split_domain(".com").is_err());
    assert_eq!(
        split_domain("ACME.IO.").expect("split"),
        ("acme".to_string(), "io".to_string())
    );
}

#[test]
fn error_text_maps_to_the_narrow_variants() {
    assert!(matches!(map_error("Invalid API key"), RegistrarError::Credentials(_)));
    assert!(matches!(map_error("Rate limit reached"), RegistrarError::RateLimited(_, 60)));
    assert!(matches!(map_error("Domain is already registered"), RegistrarError::Unavailable(_, _)));
    assert!(matches!(map_error("Odd failure"), RegistrarError::Rejected(_, _)));
}

#[test]
fn a_record_line_carries_type_and_value() {
    let record = DnsRecord::new("_dmarc", RecordType::Txt, "v=DMARC1; p=reject");
    assert_eq!(record_line(&record), "TXT=v=DMARC1; p=reject");
}

#[test]
fn a_zone_entry_parses_back_into_a_record() {
    let node = serde_json::json!({
        "recordId": "r1", "host": "@", "recordType": "A", "recordLine": "A=203.0.113.7", "ttl": 600,
    });
    let parsed = parse_entry(&node).expect("record");
    assert_eq!(parsed.record_id.as_deref(), Some("r1"));
    assert_eq!(parsed.record_type, RecordType::A);
    assert_eq!(parsed.value, "203.0.113.7");
    assert_eq!(parsed.ttl, 600);
}

#[test]
fn a_record_line_without_the_type_prefix_is_still_read() {
    let node = serde_json::json!({ "host": "@", "recordType": "TXT", "recordLine": "v=spf1 -all" });
    let parsed = parse_entry(&node).expect("record");
    assert_eq!(parsed.value, "v=spf1 -all");
}

#[test]
fn an_unsupported_record_type_is_skipped() {
    assert!(parse_entry(&serde_json::json!({"recordType": "SVCB", "recordLine": "SVCB=1 alpn=h2"})).is_none());
}

#[test]
fn the_adapter_edits_records_individually() {
    let info = Dynadot::new().info();
    assert!(info.manages_dns);
    assert!(!info.replaces_whole_zone);
    assert!(info.manages_dnssec);
}

#[tokio::test]
async fn renewal_pricing_is_flat() {
    let prices = Dynadot::new().tld_list(&Credential::default()).await.expect("prices");
    for price in &prices {
        assert_eq!(price.registration_cents, price.renewal_cents, "{}", price.tld);
    }
}
