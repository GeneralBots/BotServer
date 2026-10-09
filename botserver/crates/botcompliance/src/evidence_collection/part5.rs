use super::*;

impl EvidenceCollectionService {
    pub async fn get_evidence_by_framework(&self, framework: ComplianceFramework) -> Vec<EvidenceItem> {
        let evidence_store = self.evidence.read().await;
        evidence_store.values().filter(|e| e.frameworks.contains(&framework)).cloned().collect()
    }

    pub async fn update_evidence_status(&self, id: Uuid, status: EvidenceStatus, reviewer_id: Option<Uuid>) -> Result<EvidenceItem, CollectionError> {
        let mut evidence_store = self.evidence.write().await;
        let evidence = evidence_store.get_mut(&id).ok_or_else(|| CollectionError::NotFound("Evidence not found".to_string()))?;
        evidence.status = status;
        evidence.updated_at = Utc::now();
        if reviewer_id.is_some() {
            evidence.reviewed_at = Some(Utc::now());
            evidence.reviewed_by = reviewer_id;
        }
        Ok(evidence.clone())
    }

    pub async fn collect_automated_evidence(&self, control_id: &str) -> Result<EvidenceItem, CollectionError> {
        let mappings = self.control_mappings.read().await;
        let mapping = mappings.get(control_id).ok_or_else(|| CollectionError::NotFound("Control mapping not found".to_string()))?.clone();
        drop(mappings);

        if !mapping.automated_collection {
            return Err(CollectionError::NotAutomated("This control does not support automated collection".to_string()));
        }

        let mut collected_data = HashMap::new();
        collected_data.insert("control_id".to_string(), control_id.to_string());
        collected_data.insert("collection_time".to_string(), Utc::now().to_rfc3339());
        collected_data.insert("automated".to_string(), "true".to_string());

        for source in &mapping.collection_sources {
            if source.enabled {
                let source_data = self.collect_from_source(source).await?;
                collected_data.extend(source_data);
            }
        }

        let evidence = EvidenceItem {
            id: Uuid::new_v4(),
            title: format!("Automated Evidence - {}", mapping.control_name),
            description: Some(format!("Automatically collected evidence for control {}", control_id)),
            evidence_type: mapping.required_evidence_types.first().cloned().unwrap_or(EvidenceType::Log),
            status: EvidenceStatus::PendingReview,
            frameworks: vec![mapping.framework.clone()],
            control_ids: vec![control_id.to_string()],
            tsc_categories: mapping.tsc_category.iter().cloned().collect(),
            collection_method: CollectionMethod::Automated,
            collected_at: Utc::now(),
            collected_by: None,
            reviewed_at: None,
            reviewed_by: None,
            valid_from: Utc::now(),
            valid_until: Utc::now() + Duration::days(i64::from(mapping.collection_frequency_days)),
            file_path: None,
            file_hash: None,
            file_size_bytes: None,
            content_type: Some("application/json".to_string()),
            source_system: Some("automated_collection".to_string()),
            source_query: None,
            metadata: collected_data,
            tags: vec!["automated".to_string(), control_id.to_string()],
            version: 1,
            previous_version_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut evidence_store = self.evidence.write().await;
        evidence_store.insert(evidence.id, evidence.clone());
        Ok(evidence)
    }

    async fn collect_from_source(&self, source: &CollectionSource) -> Result<HashMap<String, String>, CollectionError> {
        let mut data = HashMap::new();
        data.insert("source_name".to_string(), source.source_name.clone());
        data.insert("source_type".to_string(), format!("{:?}", source.source_type));
        data.insert("collected_at".to_string(), Utc::now().to_rfc3339());
        Ok(data)
    }
}
