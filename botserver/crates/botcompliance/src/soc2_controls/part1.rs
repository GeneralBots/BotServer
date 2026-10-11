//! Default SOC 2 controls, part 1 of 2.
//! Data-only module split from soc2.rs for file-size limits.

use crate::soc2::{
    EvidenceType, ImplementationStatus, Soc2Control, TestFrequency, TrustServiceCategory,
};

pub(super) fn controls() -> Vec<Soc2Control> {
    vec![
        Soc2Control {
            id: "CC1.1".to_string(),
            category: TrustServiceCategory::Security,
            name: "Control Environment".to_string(),
            description: "The entity demonstrates a commitment to integrity and ethical values"
                .to_string(),
            criteria: vec![
                "Code of conduct established".to_string(),
                "Ethics training completed".to_string(),
                "Whistleblower policy in place".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Policy, EvidenceType::Certificate],
            test_frequency: TestFrequency::Annually,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC2.1".to_string(),
            category: TrustServiceCategory::Security,
            name: "Communication and Information".to_string(),
            description:
                "The entity obtains or generates relevant information to support internal control"
                    .to_string(),
            criteria: vec![
                "Security policies communicated".to_string(),
                "Security awareness training".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Policy, EvidenceType::Certificate],
            test_frequency: TestFrequency::Quarterly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC3.1".to_string(),
            category: TrustServiceCategory::Security,
            name: "Risk Assessment".to_string(),
            description:
                "The entity specifies objectives with sufficient clarity to identify risks"
                    .to_string(),
            criteria: vec![
                "Risk assessment performed".to_string(),
                "Risk register maintained".to_string(),
                "Risk treatment plans defined".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Report, EvidenceType::Policy],
            test_frequency: TestFrequency::Annually,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC4.1".to_string(),
            category: TrustServiceCategory::Security,
            name: "Monitoring Activities".to_string(),
            description: "The entity selects and develops monitoring activities".to_string(),
            criteria: vec![
                "Continuous monitoring implemented".to_string(),
                "Security metrics defined".to_string(),
                "Alert thresholds configured".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Configuration, EvidenceType::Logs],
            test_frequency: TestFrequency::Monthly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC5.1".to_string(),
            category: TrustServiceCategory::Security,
            name: "Control Activities".to_string(),
            description: "The entity selects and develops control activities".to_string(),
            criteria: vec![
                "Access controls implemented".to_string(),
                "Change management process".to_string(),
                "Segregation of duties".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Configuration, EvidenceType::Procedure],
            test_frequency: TestFrequency::Quarterly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC6.1".to_string(),
            category: TrustServiceCategory::Security,
            name: "Logical and Physical Access Controls".to_string(),
            description: "The entity implements logical access security measures".to_string(),
            criteria: vec![
                "Authentication mechanisms".to_string(),
                "MFA for privileged access".to_string(),
                "Role-based access control".to_string(),
                "Access reviews performed".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![
                EvidenceType::Configuration,
                EvidenceType::Logs,
                EvidenceType::Report,
            ],
            test_frequency: TestFrequency::Monthly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC6.2".to_string(),
            category: TrustServiceCategory::Security,
            name: "User Registration and Authorization".to_string(),
            description:
                "Prior to issuing system credentials, the entity registers authorized users"
                    .to_string(),
            criteria: vec![
                "User provisioning process".to_string(),
                "Access request workflow".to_string(),
                "Manager approval required".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Procedure, EvidenceType::Logs],
            test_frequency: TestFrequency::Monthly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC6.3".to_string(),
            category: TrustServiceCategory::Security,
            name: "User Deprovisioning".to_string(),
            description: "The entity removes access when no longer required".to_string(),
            criteria: vec![
                "Termination checklist".to_string(),
                "Automated deprovisioning".to_string(),
                "Access removal within 24 hours".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Procedure, EvidenceType::Logs],
            test_frequency: TestFrequency::Monthly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC6.6".to_string(),
            category: TrustServiceCategory::Security,
            name: "System Boundary Protection".to_string(),
            description: "The entity implements measures to protect system boundaries".to_string(),
            criteria: vec![
                "Firewall configuration".to_string(),
                "Network segmentation".to_string(),
                "Intrusion detection".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Configuration, EvidenceType::Logs],
            test_frequency: TestFrequency::Monthly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC6.7".to_string(),
            category: TrustServiceCategory::Security,
            name: "Encryption".to_string(),
            description:
                "The entity restricts transmission and movement of data to authorized parties"
                    .to_string(),
            criteria: vec![
                "TLS 1.3 for transit".to_string(),
                "AES-256 for data at rest".to_string(),
                "Key management procedures".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Configuration, EvidenceType::Certificate],
            test_frequency: TestFrequency::Quarterly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC7.1".to_string(),
            category: TrustServiceCategory::Security,
            name: "System Operations".to_string(),
            description: "The entity detects and monitors security events".to_string(),
            criteria: vec![
                "Security monitoring configured".to_string(),
                "Log aggregation".to_string(),
                "Alerting mechanisms".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Configuration, EvidenceType::Logs],
            test_frequency: TestFrequency::Continuous,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC7.2".to_string(),
            category: TrustServiceCategory::Security,
            name: "Incident Management".to_string(),
            description: "The entity monitors system components for anomalies".to_string(),
            criteria: vec![
                "Incident response plan".to_string(),
                "Incident classification".to_string(),
                "Escalation procedures".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Policy, EvidenceType::Procedure],
            test_frequency: TestFrequency::Quarterly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC7.3".to_string(),
            category: TrustServiceCategory::Security,
            name: "Incident Response".to_string(),
            description: "The entity evaluates security events and responds appropriately"
                .to_string(),
            criteria: vec![
                "Response procedures documented".to_string(),
                "Communication protocols".to_string(),
                "Post-incident review".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Procedure, EvidenceType::Report],
            test_frequency: TestFrequency::Quarterly,
            last_tested: None,
            next_test_due: None,
        },
        Soc2Control {
            id: "CC7.4".to_string(),
            category: TrustServiceCategory::Security,
            name: "Incident Recovery".to_string(),
            description: "The entity responds to identified security incidents".to_string(),
            criteria: vec![
                "Recovery procedures".to_string(),
                "Business continuity plan".to_string(),
                "Disaster recovery testing".to_string(),
            ],
            implementation_status: ImplementationStatus::FullyImplemented,
            evidence_types: vec![EvidenceType::Procedure, EvidenceType::Report],
            test_frequency: TestFrequency::Annually,
            last_tested: None,
            next_test_due: None,
        },
    ]
}
