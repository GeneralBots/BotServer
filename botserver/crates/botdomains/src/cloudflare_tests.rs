//! Unit tests for `super` (cloudflare.rs).
//!
//! Split out of `cloudflare.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

use crate::Registrar;

fn record() -> DnsRecord {
    DnsRecord::new("", RecordType::A, "203.0.113.7")
}

#[test]
fn an_unconfigured_adapter_refuses_before_any_request() {
    let bare = Cloudflare::default();
    assert!(bare.require_account().is_err());
    assert!(Cloudflare::with_account("acct").require_account().is_ok());
}

#[test]
fn the_adapter_manages_dnssec_and_edits_records_individually() {
    let info = Cloudflare::with_account("acct").info();
    assert!(info.manages_dns);
    assert!(info.manages_dnssec);
    assert!(!info.replaces_whole_zone);
}

#[test]
fn a_hostname_is_percent_encoded_for_a_query_string() {
    assert_eq!(encode("acme.com"), "acme.com");
    assert_eq!(encode("a b"), "a%20b");
    assert_eq!(encode("a&b=c"), "a%26b%3Dc");
}

#[test]
fn envelope_codes_map_to_the_narrow_variants() {
    let auth = serde_json::json!({"success": false, "errors": [{"code": 10000, "message": "bad token"}]});
    assert!(matches!(envelope_error(&auth), RegistrarError::Credentials(_)));
    let throttled = serde_json::json!({"success": false, "errors": [{"code": 1002, "message": "slow down"}]});
    assert!(matches!(envelope_error(&throttled), RegistrarError::RateLimited(_, 60)));
    let missing = serde_json::json!({"success": false, "errors": [{"code": 8103, "message": "no zone"}]});
    assert!(matches!(envelope_error(&missing), RegistrarError::NotFound(_, _)));
    let other = serde_json::json!({"success": false, "errors": [{"code": 9999, "message": "odd"}]});
    assert!(matches!(envelope_error(&other), RegistrarError::Rejected(_, _)));
}

#[test]
fn an_apex_record_is_sent_as_at() {
    let payload = record_payload(&record());
    assert_eq!(payload["name"], "@");
    assert_eq!(payload["type"], "A");
}

#[test]
fn txt_records_are_never_proxied() {
    // A DMARC or SPF record behind the Cloudflare proxy is never read by a
    // mail server, so `proxied` must stay false for every record type here.
    let payload = record_payload(&DnsRecord::new("_dmarc", RecordType::Txt, "v=DMARC1; p=reject"));
    assert_eq!(payload["proxied"], false);
    assert_eq!(payload["type"], "TXT");
}

#[test]
fn a_cloudflare_record_parses_back_into_the_platform_shape() {
    let node = serde_json::json!({
        "id": "rec1", "type": "A", "name": "acme.com", "content": "203.0.113.7", "ttl": 1,
    });
    let parsed = record_from(&node).expect("record");
    assert_eq!(parsed.record_id.as_deref(), Some("rec1"));
    assert_eq!(parsed.record_type, RecordType::A);
    assert_eq!(parsed.host, "acme.com");
}

#[test]
fn an_unsupported_cloudflare_type_is_skipped() {
    assert!(record_from(&serde_json::json!({"type": "HTTPS", "name": "acme.com"})).is_none());
}

#[tokio::test]
async fn renewal_pricing_is_at_cost_and_flat() {
    let prices = Cloudflare::with_account("acct").tld_list(&Credential::default()).await.expect("prices");
    assert!(!prices.is_empty());
    for price in &prices {
        assert_eq!(price.registration_cents, price.renewal_cents, "{}", price.tld);
    }
}
