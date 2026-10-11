//! Unit tests for `super` (ovh.rs).
//!
//! Split out of `ovh.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

fn spec(cores: u32, ram: u32) -> MachineSpec {
    MachineSpec { cpu_cores: cores, ram_gb: ram, ..MachineSpec::default() }
}

fn sign(ts: &str, method: &str, url: &str, body: &str) -> String {
    OvhProvider::signature("secret", "key", ts, method, url, body, "application/json")
        .expect("a non-empty secret always signs")
}

#[test]
fn the_signature_is_stable_for_fixed_inputs() {
    let first = sign("1700000000", "GET", "https://eu.api.ovh.com/1.0/x", "");
    let second = sign("1700000000", "GET", "https://eu.api.ovh.com/1.0/x", "");
    assert_eq!(first, second);
    assert_eq!(first.len(), 40, "HMAC-SHA1 is 20 bytes, hex-encoded");
}

#[test]
fn the_signature_changes_with_every_signed_field() {
    let base = |ts: &str, method: &str, url: &str, body: &str| sign(ts, method, url, body);
    let reference = base("1700000000", "GET", "https://eu.api.ovh.com/1.0/x", "");
    assert_ne!(reference, base("1700000001", "GET", "https://eu.api.ovh.com/1.0/x", ""));
    assert_ne!(reference, base("1700000000", "POST", "https://eu.api.ovh.com/1.0/x", ""));
    assert_ne!(reference, base("1700000000", "GET", "https://eu.api.ovh.com/1.0/y", ""));
    assert_ne!(reference, base("1700000000", "GET", "https://eu.api.ovh.com/1.0/x", "{}"));
}

#[test]
fn a_different_secret_yields_a_different_signature() {
    let with = |secret: &str| {
        OvhProvider::signature(
            secret, "key", "1700000000", "GET", "https://eu.api.ovh.com/", "", "application/json",
        )
    };
    assert_ne!(with("secret-a").expect("signed"), with("secret-b").expect("signed"));
}

#[test]
fn the_method_is_uppercased_before_signing() {
    let lower = OvhProvider::signature(
        "s", "k", "1", "get", "https://eu.api.ovh.com/", "", "application/json",
    );
    let upper = OvhProvider::signature(
        "s", "k", "1", "GET", "https://eu.api.ovh.com/", "", "application/json",
    );
    assert_eq!(lower.expect("signed"), upper.expect("signed"));
}

#[test]
fn signing_without_a_secret_is_refused() {
    assert!(matches!(
        OvhProvider::signature("", "key", "1", "GET", "https://eu.api.ovh.com/", "", "application/json"),
        Err(ProviderError::Auth(_))
    ));
}

#[test]
fn credentials_must_be_key_slash_secret() {
    assert!(OvhProvider::parse_credential("key/secret").is_ok());
    assert!(matches!(
        OvhProvider::parse_credential("no-separator"),
        Err(ProviderError::Auth(_))
    ));
    assert!(matches!(
        OvhProvider::parse_credential("/secret"),
        Err(ProviderError::Auth(_))
    ));
    assert!(matches!(
        OvhProvider::parse_credential("key/"),
        Err(ProviderError::Auth(_))
    ));
}

#[test]
fn datacentre_gpus_map_to_the_t1_family() {
    for gpu in ["H100", "H200", "A100", "L40S", "A10", "RTX A4000"] {
        assert_eq!(OvhProvider::gpu_flavor(Some(gpu)), "t1-45", "{gpu}");
    }
}

#[test]
fn an_unrecognised_gpu_falls_back_to_a_general_accelerator() {
    // Falling back to a datacentre shape would quote a product the account
    // may not be entitled to.
    assert_eq!(OvhProvider::gpu_flavor(Some("RTX 4090")), "g1-15");
    assert_eq!(OvhProvider::gpu_flavor(None), "s1-2");
}

#[test]
fn cpu_specs_map_to_the_s_family() {
    assert_eq!(OvhProvider::cpu_flavor(&spec(2, 8)).expect("flavor"), "s1-2");
    assert_eq!(OvhProvider::cpu_flavor(&spec(4, 16)).expect("flavor"), "s2-4");
    assert_eq!(OvhProvider::cpu_flavor(&spec(16, 64)).expect("flavor"), "s1-16");
}

#[test]
fn a_request_above_the_largest_plan_is_refused() {
    assert!(matches!(
        OvhProvider::cpu_flavor(&spec(64, 256)),
        Err(ProviderError::Capacity(_))
    ));
}

#[test]
fn no_request_is_ever_mapped_to_a_smaller_flavor() {
    // s1-8 is 8 vCore/32 GB; an earlier ordering sent a 16-core request
    // there, which quietly under-provisioned the tenant.
    let flavor_sizes = |flavor: &str| -> (u32, u32) {
        match flavor {
            "s1-2" => (2, 8),
            "s2-4" => (4, 16),
            "s2-8" => (8, 32),
            _ => (16, 64),
        }
    };
    for cores in 1..=32u32 {
        for ram in [1u32, 4, 8, 16, 32, 64] {
            let request = spec(cores, ram);
            match OvhProvider::cpu_flavor(&request) {
                Ok(flavor) => {
                    let (plan_cores, plan_ram) = flavor_sizes(flavor);
                    assert!(
                        plan_cores >= cores && plan_ram >= ram,
                        "{cores} cores/{ram} GB resolved to {flavor} ({plan_cores}/{plan_ram})"
                    );
                }
                Err(err) => assert!(matches!(err, ProviderError::Capacity(_)), "{err}"),
            }
        }
    }
}

#[test]
fn service_names_follow_the_region_prefix() {
    assert_eq!(OvhProvider::service_name("GRA"), "ovh:cloud:projectEu");
    assert_eq!(OvhProvider::service_name("BHS"), "ovh:cloud:projectCa");
    assert_eq!(OvhProvider::service_name("SGP1"), "ovh:cloud:projectAp");
    assert_eq!(OvhProvider::service_name(""), "ovh:cloud:projectEu");
}
