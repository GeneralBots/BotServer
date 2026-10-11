//! Unit tests for `super` (desec.rs).
//!
//! Split out of `desec.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

use crate::Registrar;

fn record() -> DnsRecord {
    DnsRecord::new("", RecordType::A, "203.0.113.7")
}

#[test]
fn the_adapter_declares_no_tlds_because_it_registers_nothing() {
    let info = Desec::default().info();
    assert!(info.supported_tlds.is_empty(), "deSEC is not a registrar");
    assert!(info.manages_dns);
    assert!(info.manages_dnssec);
}

#[test]
fn errors_map_to_the_narrow_variants() {
    assert!(matches!(map_error("RRset not found"), RegistrarError::NotFound(_, _)));
    assert!(matches!(map_error("Invalid token"), RegistrarError::Credentials(_)));
    assert!(matches!(map_error("Rate limit reached"), RegistrarError::RateLimited(_, 60)));
    assert!(matches!(map_error("Unsupported type"), RegistrarError::Unsupported(_, _)));
    assert!(matches!(map_error("Odd failure"), RegistrarError::Rejected(_, _)));
}

#[test]
fn an_a_record_is_sent_as_a_single_valued_rrset() {
    let payload = Desec::payload(&record());
    assert_eq!(payload["type"], "A");
    assert_eq!(payload["rrset"], serde_json::json!(["203.0.113.7"]));
}

#[test]
fn a_long_txt_value_is_split_into_chunks_and_rejoined() {
    let spf = DnsRecord::new("", RecordType::Txt, "v=spf1 include:mail.example.net ~all");
    let payload = Desec::payload(&spf);
    let chunks = payload["rrset"].as_array().expect("chunks");
    assert!(chunks.len() > 1, "a long TXT must be chunked on the wire");

    let node = serde_json::json!({ "ttl": 300, "rrset": chunks.clone() });
    let parsed = parse_rrset(&node, RecordType::Txt).expect("record");
    assert_eq!(parsed.value, "v=spf1 include:mail.example.net ~all");
}

#[test]
fn an_a_rrset_reads_back_as_one_value() {
    let node = serde_json::json!({ "ttl": 300, "rrset": ["203.0.113.7"] });
    let parsed = parse_rrset(&node, RecordType::A).expect("record");
    assert_eq!(parsed.value, "203.0.113.7");
}

#[test]
fn an_unbound_adapter_refuses_a_zone_operation() {
    assert!(Desec::default().require_zone().is_err());
    assert!(Desec::with_zone("acme.com").require_zone().is_ok());
}

#[tokio::test]
async fn the_registration_methods_report_they_are_dns_only() {
    let adapter = Desec::with_zone("acme.com");
    let credential = Credential::new("token", "");
    for result in [
        adapter.check("acme.com", &credential).await.err(),
        adapter.register(&DomainSpec::one_year("acme.com"), &credential).await.err(),
        adapter.renew("acme.com", 1, &credential).await.err(),
        adapter.transfer("acme.com", &credential).await.err(),
    ] {
        assert!(matches!(result, Some(RegistrarError::Unsupported(_, _))), "{result:?}");
    }
}

#[tokio::test]
async fn delegation_points_at_the_desec_nameservers() {
    let err = Desec::with_zone("acme.com")
        .set_nameservers("acme.com", &["ns1.desec.io".to_string()], &Credential::new("t", ""))
        .await
        .unwrap_err();
    match err {
        RegistrarError::Unsupported(_, message) => {
            assert!(message.contains("ns1.desec.io"), "{message}");
        }
        other => panic!("expected Unsupported, got {other}"),
    }
}

#[tokio::test]
async fn an_empty_nameserver_list_is_rejected() {
    let err = Desec::with_zone("acme.com")
        .set_nameservers("acme.com", &[], &Credential::new("t", ""))
        .await
        .unwrap_err();
    assert!(matches!(err, RegistrarError::Rejected(_, _)));
}

#[tokio::test]
async fn a_missing_token_is_refused_before_any_request() {
    let err = Desec::with_zone("acme.com")
        .write_record(&record(), &Credential::default())
        .await
        .unwrap_err();
    assert!(matches!(err, RegistrarError::Credentials(_)));
}

#[tokio::test]
async fn the_tld_list_is_empty_because_nothing_is_sold() {
    assert!(Desec::with_zone("acme.com")
        .tld_list(&Credential::default())
        .await
        .expect("list")
        .is_empty());
}
