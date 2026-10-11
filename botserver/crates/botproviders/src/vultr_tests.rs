//! Unit tests for `super` (vultr.rs).
//!
//! Split out of `vultr.rs` to keep the file inside the project size
//! limit; the production code stays beside it.

use super::*;

fn plan(cores: u32, ram: u32, gpu: Option<&str>) -> Result<&'static str, ProviderError> {
    VultrProvider::map_plan(&spec(cores, ram, gpu))
}

fn spec(cores: u32, ram: u32, gpu: Option<&str>) -> MachineSpec {
    MachineSpec {
        cpu_cores: cores,
        ram_gb: ram,
        gpu_type: gpu.map(str::to_string),
        ..MachineSpec::default()
    }
}

#[test]
fn a_gpu_request_reaches_a_gpu_plan() {
    // Regression: the GPU arms used to sit below the exact CPU arms, so a
    // 4-core/16-GB GPU request silently got a CPU plan.
    assert_eq!(plan(4, 16, Some("RTX 4090")).expect("plan"), "gpu-4c-16gb");
    assert_eq!(plan(8, 32, Some("A100")).expect("plan"), "gpu-8c-32gb");
    assert_eq!(plan(2, 8, Some("RTX 3090")).expect("plan"), "gpu-2c-8gb");
}

#[test]
fn a_cpu_request_still_reaches_a_cpu_plan() {
    assert_eq!(plan(4, 16, None).expect("plan"), "vhp-4c-16gb");
    assert_eq!(plan(2, 4, None).expect("plan"), "vhp-2c-4gb");
    assert_eq!(plan(16, 64, None).expect("plan"), "vhp-16c-64gb");
}

#[test]
fn an_undersized_spec_rounds_up_rather_than_down() {
    assert_eq!(plan(1, 1, None).expect("plan"), "vhp-2c-4gb");
    assert_eq!(plan(0, 0, None).expect("plan"), "vhp-2c-4gb");
}

#[test]
fn a_request_above_the_largest_plan_is_refused() {
    assert!(matches!(plan(32, 128, None), Err(ProviderError::Capacity(_))));
    // No GPU plan covers 16 cores, so this must be refused rather than
    // silently handed a CPU plan.
    assert!(matches!(
        plan(16, 64, Some("A100")),
        Err(ProviderError::Capacity(_))
    ));
}

/// The plan a request resolves to must never be smaller than what it asked
/// for — under-provisioning is how a tenant discovers they were short-changed.
#[test]
fn no_request_is_ever_mapped_to_a_smaller_plan() {
    let plan_sizes = |plan: &str| -> (u32, u32) {
        match plan {
            "vhp-2c-4gb" => (2, 4),
            "gpu-2c-8gb" => (2, 8),
            "vhp-2c-8gb" => (2, 8),
            "gpu-4c-16gb" => (4, 16),
            "vhp-4c-16gb" => (4, 16),
            "vhp-6c-24gb" => (6, 24),
            "gpu-8c-32gb" => (8, 32),
            "vhp-8c-32gb" => (8, 32),
            _ => (16, 64),
        }
    };
    for cores in 1..=32u32 {
        for ram in [1u32, 2, 4, 8, 16, 24, 32, 64] {
            for gpu in [None, Some("RTX 4090")] {
                match plan(cores, ram, gpu) {
                    Ok(chosen) => {
                        let (plan_cores, plan_ram) = plan_sizes(chosen);
                        assert!(
                            plan_cores >= cores && plan_ram >= ram,
                            "{cores} cores/{ram} GB (gpu={gpu:?}) resolved to {chosen}"
                        );
                    }
                    Err(err) => assert!(matches!(err, ProviderError::Capacity(_)), "{err}"),
                }
            }
        }
    }
}

#[test]
fn the_adapter_declares_an_os_id() {
    assert!(!VultrProvider::OS_ID.is_empty());
}
