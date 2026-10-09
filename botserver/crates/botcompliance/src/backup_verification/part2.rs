use super::*;

impl Default for VerificationPolicy {
    fn default() -> Self {
        Self {
            auto_verify: true,
            verification_methods: vec![
                VerificationMethod::ChecksumValidation,
                VerificationMethod::MetadataCheck,
            ],
            verify_after_backup: true,
            periodic_verification_days: 7,
            restore_test_frequency_days: Some(30),
            sample_data_percentage: Some(5),
            fail_on_verification_error: true,
        }
    }
}
