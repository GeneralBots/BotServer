//! Unit tests for `super` (namecheap.rs).
//!
//! Split out of `namecheap.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

use crate::Registrar;

const ZONE_XML: &str = r#"<?xml version="1.0"?>
<ApiResponse Status="OK">
  <Errors />
  <CommandResponseType>getList</CommandResponseType>
  <Domain>acme.com</Domain>
  <DomainGetListResult>
<host><Host>@</Host><Type>A</Type><Address>203.0.113.7</Address><TTL>1800</TTL></host>
<host><Host>_dmarc</Host><Type>TXT</Type><Address>v=DMARC1; p=reject</Address><TTL>1800</TTL></host>
  </DomainGetListResult>
</ApiResponse>"#;

fn zone_records() -> Vec<DnsRecord> {
    records_from(&xml::parse(SERVICE, ZONE_XML).expect("parsed"))
}

#[test]
fn the_adapter_declares_the_zone_replace_hazard() {
    let info = Namecheap::default().info();
    assert!(info.replaces_whole_zone, "the hazard must be visible in the catalog");
    assert!(info.manages_dns);
}

#[test]
fn a_zone_response_parses_into_records() {
    let records = zone_records();
    assert_eq!(records.len(), 2, "{records:?}");
    assert_eq!(records[0].host, "@");
    assert_eq!(records[0].record_type, RecordType::A);
    assert_eq!(records[0].value, "203.0.113.7");
    assert_eq!(records[1].host, "_dmarc");
    assert_eq!(records[1].record_type, RecordType::Txt);
}

#[test]
fn mx_records_are_not_duplicated_by_their_hostattr() {
    let xml_body = r#"<ApiResponse Status="OK"><Errors/><DomainGetListResult>
        <host><Host>@</Host><Type>MX</Type><Address>mail1.example.net</Address><TTL>1800</TTL></host>
        <hostattr><Host>@</Host><Type>MX</Type><Value>10</Value></hostattr>
    </DomainGetListResult></ApiResponse>"#;
    let records = records_from(&xml::parse(SERVICE, xml_body).expect("parsed"));
    assert_eq!(records.len(), 1, "the hostattr block duplicates the MX record");
    assert_eq!(records[0].record_type, RecordType::Mx);
}

#[test]
fn an_unsupported_record_type_is_skipped_not_mangled() {
    let xml_body = r#"<ApiResponse Status="OK"><Errors/><DomainGetListResult>
        <host><Host>_https._tcp</Host><Type>HTTPS</Type><Address>alpn=h2</Address></host>
        <host><Host>@</Host><Type>A</Type><Address>203.0.113.7</Address></host>
    </DomainGetListResult></ApiResponse>"#;
    let records = records_from(&xml::parse(SERVICE, xml_body).expect("parsed"));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].record_type, RecordType::A);
}

#[test]
fn error_numbers_map_to_the_narrow_variants() {
    assert!(matches!(map_error("20110: Invalid API key"), RegistrarError::Credentials(_)));
    assert!(matches!(map_error("20118: Too many requests"), RegistrarError::RateLimited(_, 60)));
    assert!(matches!(
        map_error("2017: Domain already registered"),
        RegistrarError::Unavailable(_, _)
    ));
    assert!(matches!(map_error("2019: Not supported"), RegistrarError::Unsupported(_, _)));
    assert!(matches!(map_error("9999: Odd"), RegistrarError::Rejected(_, _)));
}

#[test]
fn adding_one_record_would_not_delete_the_zone() {
    let current = zone_records();
    let mut desired = current.clone();
    desired.push(DnsRecord::new("www", RecordType::Cname, "app.example.net"));
    let diff = zone::diff(&current, &desired);
    assert!(diff.ensure_no_silent_deletions("Namecheap").is_ok());
}

#[test]
fn a_write_that_drops_the_mx_record_is_refused() {
    let current = vec![
        DnsRecord::new("@", RecordType::Mx, "mail.example.net"),
        DnsRecord::new("@", RecordType::A, "203.0.113.7"),
    ];
    let desired = vec![DnsRecord::new("@", RecordType::A, "203.0.113.7")];
    let diff = zone::diff(&current, &desired);
    assert!(diff.ensure_no_silent_deletions("Namecheap").is_err());
}

#[test]
fn an_incomplete_domain_is_refused() {
    assert!(split_domain("acme").is_err());
    assert!(split_domain(".com").is_err());
    assert_eq!(
        split_domain("ACME.COM").expect("split"),
        ("acme".to_string(), "com".to_string())
    );
}

#[test]
fn nameserver_parameters_are_positional_and_bounded() {
    assert_eq!(nameserver_param(0), "ns1");
    assert_eq!(nameserver_param(2), "ns3");
    assert_eq!(nameserver_param(9), "ns4");
}

#[tokio::test]
async fn a_missing_client_ip_is_refused_before_any_request() {
    let err = Namecheap::default()
        .check("acme.com", &Credential::new("k", "s"))
        .await
        .unwrap_err();
    assert!(matches!(err, RegistrarError::Credentials(_)));
    assert!(err.to_string().contains("client IP"), "{err}");
}

#[tokio::test]
async fn per_record_delete_is_reported_as_unsupported() {
    let err = Namecheap::sandbox()
        .delete_dns_record("acme.com", "123", &Credential::new("k", "s"))
        .await
        .unwrap_err();
    assert!(matches!(err, RegistrarError::Unsupported(_, _)));
}

#[tokio::test]
async fn renewal_pricing_is_marked_up_against_registration() {
    let prices = Namecheap::sandbox().tld_list(&Credential::default()).await.expect("prices");
    assert!(!prices.is_empty());
    for price in &prices {
        assert!(
            price.renewal_cents > price.registration_cents,
            "{} renews flat, which is not Namecheap's shape",
            price.tld
        );
    }
}
