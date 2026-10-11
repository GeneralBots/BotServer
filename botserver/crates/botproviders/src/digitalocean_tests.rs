//! Unit tests for `super` (digitalocean.rs).
//!
//! Split out of `digitalocean.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

fn spec(cores: u32, ram: u32) -> MachineSpec {
    MachineSpec { cpu_cores: cores, ram_gb: ram, disk_gb: 25, ..MachineSpec::default() }
}

fn slug(cores: u32, ram: u32) -> Result<&'static str, ProviderError> {
    DigitalOceanProvider::slug(&spec(cores, ram))
}

#[test]
fn small_specs_land_on_the_published_slugs() {
    assert_eq!(slug(1, 1).expect("slug"), "s-1vcpu-1gb");
    assert_eq!(slug(1, 2).expect("slug"), "s-1vcpu-2gb");
    assert_eq!(slug(2, 4).expect("slug"), "s-2vcpu-4gb");
    assert_eq!(slug(4, 8).expect("slug"), "s-4vcpu-8gb");
    assert_eq!(slug(8, 16).expect("slug"), "s-8vcpu-16gb");
}

#[test]
fn a_requirement_rounds_up_rather_than_down() {
    assert_eq!(slug(3, 8).expect("slug"), "s-4vcpu-8gb");
    assert_eq!(slug(9, 16).expect("slug"), "s-16vcpu-32gb");
}

#[test]
fn the_largest_slug_is_reachable_and_nothing_beyond_it_is() {
    assert_eq!(slug(16, 32).expect("slug"), DigitalOceanProvider::LARGEST);
    assert!(matches!(
        slug(32, 128),
        Err(ProviderError::Capacity(_))
    ));
}

#[test]
fn no_request_is_ever_mapped_to_a_smaller_droplet() {
    let slug_sizes = |slug: &str| -> (u32, u32) {
        match slug {
            "s-1vcpu-1gb" => (1, 1),
            "s-1vcpu-2gb" => (1, 2),
            "s-2vcpu-4gb" => (2, 4),
            "s-4vcpu-8gb" => (4, 8),
            "s-8vcpu-16gb" => (8, 16),
            _ => (16, 32),
        }
    };
    for cores in 1..=32u32 {
        for ram in [1u32, 2, 4, 8, 16, 32] {
            match slug(cores, ram) {
                Ok(slug) => {
                    let (plan_cores, plan_ram) = slug_sizes(slug);
                    assert!(
                        plan_cores >= cores && plan_ram >= ram,
                        "{cores} cores/{ram} GB resolved to {slug} ({plan_cores}/{plan_ram})"
                    );
                }
                Err(err) => assert!(matches!(err, ProviderError::Capacity(_)), "{err}"),
            }
        }
    }
}

#[test]
fn region_slugs_resolve_and_default_to_nyc() {
    assert_eq!(DigitalOceanProvider::region_slug("EU"), "ams3");
    assert_eq!(DigitalOceanProvider::region_slug("sgp"), "sgp1");
    assert_eq!(DigitalOceanProvider::region_slug("US"), "nyc3");
    assert_eq!(DigitalOceanProvider::region_slug(""), "nyc3");
}

#[test]
fn the_adapter_advertises_no_gpu_line() {
    let info = DigitalOceanProvider::new().info();
    assert!(info.available_gpus.is_empty());
    assert!(info.supports_spot);
    assert_eq!(info.name, "digitalocean");
}
