//! Unit tests for `super` (oracle.rs).
//!
//! Split out of `oracle.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

fn spec(cores: u32, ram: u32) -> MachineSpec {
    MachineSpec { cpu_cores: cores, ram_gb: ram, disk_gb: 50, ..MachineSpec::default() }
}

#[test]
fn small_cpu_specs_land_on_ampere() {
    assert_eq!(OracleProvider::shape(&spec(1, 8)), "VM.Standard.A1.Flex");
    assert_eq!(OracleProvider::shape(&spec(4, 24)), "VM.Standard.A1.Flex");
}

#[test]
fn larger_specs_move_to_x86_flex_shapes() {
    assert_eq!(OracleProvider::shape(&spec(8, 32)), "VM.Standard.E4.Flex");
    assert_eq!(OracleProvider::shape(&spec(16, 64)), "VM.Standard.E5.Flex");
}

#[test]
fn a_zero_spec_does_not_panic() {
    assert_eq!(OracleProvider::shape(&spec(0, 0)), "VM.Standard.A1.Flex");
}

#[test]
fn region_ids_resolve_and_default_to_the_home_region() {
    assert_eq!(OracleProvider::region_id("US"), "us-ashburn-1");
    assert_eq!(OracleProvider::region_id("br"), "sa-saopaulo-1");
    assert_eq!(OracleProvider::region_id("EU-AMSTERDAM"), "eu-amsterdam-1");
    assert_eq!(OracleProvider::region_id("eu-zurich-2"), "eu-zurich-2");
    assert_eq!(OracleProvider::region_id("EU-ZURICH"), "eu-zurich-1");
    assert_eq!(OracleProvider::region_id("mars"), "us-ashburn-1");
    assert_eq!(OracleProvider::region_id(""), "us-ashburn-1");
}

#[test]
fn a_credential_without_a_colon_is_refused() {
    assert!(matches!(
        OracleProvider::parse_credential("no-separator"),
        Err(ProviderError::Auth(_))
    ));
    assert!(matches!(
        OracleProvider::parse_credential(":secret"),
        Err(ProviderError::Auth(_))
    ));
    assert!(matches!(
        OracleProvider::parse_credential("key:"),
        Err(ProviderError::Auth(_))
    ));
}

#[test]
fn a_credential_keeps_a_base64_secret_intact() {
    let (ocid, secret) = OracleProvider::parse_credential("ocid1.user.oc1..aaa:bXlzZWNyZXQ=")
        .expect("valid credential");
    assert_eq!(ocid, "ocid1.user.oc1..aaa");
    assert_eq!(secret, "bXlzZWNyZXQ=");
}

#[test]
fn the_auth_header_is_basic_auth_over_the_pair() {
    let header = OracleProvider::auth_header("user:pass").expect("valid credential");
    let encoded = header.strip_prefix("Basic ").expect("Basic scheme");
    let decoded = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("valid base64"),
    )
    .expect("valid utf-8");
    assert_eq!(decoded, "user:pass");
}

#[test]
fn the_adapter_advertises_no_gpu_line() {
    let info = OracleProvider::new().info();
    assert!(info.available_gpus.is_empty());
    assert!(!info.supports_spot);
    assert_eq!(info.name, "oracle");
}

#[test]
fn an_unconfigured_adapter_refuses_instead_of_guessing_ocids() {
    let bare = OracleProvider::new();
    assert!(bare.require_config().is_err());
    let partial = OracleProvider::with_config("ocid1.tenancy.oc1..aaa", "");
    assert!(partial.require_config().is_err());
    let complete = OracleProvider::with_config("ocid1.tenancy.oc1..aaa", "ocid1.image.oc1..bbb");
    assert!(complete.require_config().is_ok());
}
