//! Unit tests for `super` (hetzner.rs).
//!
//! Split out of `hetzner.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

fn spec(cores: u32, ram: u32) -> MachineSpec {
    MachineSpec { cpu_cores: cores, ram_gb: ram, disk_gb: 40, ..MachineSpec::default() }
}

fn server_type(cores: u32, ram: u32) -> Result<&'static str, ProviderError> {
    HetznerProvider::server_type(&spec(cores, ram))
}

#[test]
fn small_specs_round_onto_the_published_types() {
    assert_eq!(server_type(1, 2).expect("type"), "cx22");
    assert_eq!(server_type(2, 4).expect("type"), "cx32");
    assert_eq!(server_type(2, 8).expect("type"), "cx42");
    assert_eq!(server_type(4, 16).expect("type"), "cpx41");
    assert_eq!(server_type(16, 64).expect("type"), "ccx63");
}

#[test]
fn the_largest_type_is_reachable_and_nothing_beyond_it_is() {
    assert_eq!(server_type(16, 128).expect("type"), HetznerProvider::LARGEST);
    let too_big = server_type(64, 512);
    assert!(matches!(too_big, Err(ProviderError::Capacity(_))));
}

#[test]
fn a_zero_spec_does_not_panic() {
    assert_eq!(server_type(0, 0).expect("type"), "cx22");
}

#[test]
fn no_request_is_ever_mapped_to_a_smaller_type() {
    let type_sizes = |server_type: &str| -> (u32, u32) {
        match server_type {
            "cx22" => (1, 2),
            "cx32" => (2, 4),
            "cx42" => (2, 8),
            "cpx31" => (3, 8),
            "cx52" => (4, 8),
            "cpx41" => (4, 16),
            "cx62" => (8, 16),
            "cpx51" => (8, 32),
            "ccx63" => (16, 64),
            _ => (16, 128),
        }
    };
    for cores in 1..=32u32 {
        for ram in [1u32, 2, 4, 8, 16, 32, 64, 128] {
            match server_type(cores, ram) {
                Ok(server_type) => {
                    let (plan_cores, plan_ram) = type_sizes(server_type);
                    assert!(
                        plan_cores >= cores && plan_ram >= ram,
                        "{cores} cores/{ram} GB resolved to {server_type} ({plan_cores}/{plan_ram})"
                    );
                }
                Err(err) => {
                    // Refusing is correct; reporting it as a smaller machine
                    // is not. Both are covered by the match arm.
                    assert!(matches!(err, ProviderError::Capacity(_)), "{err}");
                }
            }
        }
    }
}

#[test]
fn region_slugs_map_to_locations() {
    assert_eq!(HetznerProvider::location_id("US"), "ash");
    assert_eq!(HetznerProvider::location_id("eu"), "hel1");
    assert_eq!(HetznerProvider::location_id("NBG"), "nbg1");
    assert_eq!(HetznerProvider::location_id("nowhere"), "hel1");
}

#[test]
fn the_adapter_advertises_no_gpu_line() {
    let info = HetznerProvider::new().info();
    assert!(info.available_gpus.is_empty());
    assert!(info.supports_spot);
    assert!(!info.regions.is_empty());
}
