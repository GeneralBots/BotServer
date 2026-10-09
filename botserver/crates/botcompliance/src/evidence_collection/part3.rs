use super::*;

impl EvidenceCollectionService {
    pub fn new() -> Self {
        let service = Self {
            evidence: Arc::new(RwLock::new(HashMap::new())),
            control_mappings: Arc::new(RwLock::new(HashMap::new())),
            collection_tasks: Arc::new(RwLock::new(Vec::new())),
            evidence_requests: Arc::new(RwLock::new(HashMap::new())),
            evidence_packages: Arc::new(RwLock::new(HashMap::new())),
            collection_schedules: Arc::new(RwLock::new(Vec::new())),
        };

        tokio::spawn({
            let service_clone = Self {
                evidence: service.evidence.clone(),
                control_mappings: service.control_mappings.clone(),
                collection_tasks: service.collection_tasks.clone(),
                evidence_requests: service.evidence_requests.clone(),
                evidence_packages: service.evidence_packages.clone(),
                collection_schedules: service.collection_schedules.clone(),
            };
            async move {
                service_clone.initialize_soc2_controls().await;
            }
        });

        service
    }
}
