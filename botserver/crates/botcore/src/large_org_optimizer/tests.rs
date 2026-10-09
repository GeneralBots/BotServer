    use super::*;

    #[test]
    pub(super) fn test_optimizer_config_default() {
        let config = OptimizerConfig::default();
        assert_eq!(config.large_org_threshold, 1000);
        assert_eq!(config.member_cache_ttl_seconds, 300);
        assert_eq!(config.permission_cache_ttl_seconds, 60);
        assert_eq!(config.batch_size, 100);
        assert_eq!(config.parallel_queries, 4);
        assert!(config.enable_query_optimization);
        assert!(config.enable_lazy_loading);
        assert!(config.enable_partitioning);
    }

    #[test]
    pub(super) fn test_is_large_organization() {
        let optimizer = LargeOrgOptimizer::default();
        assert!(!optimizer.is_large_organization(500));
        assert!(!optimizer.is_large_organization(999));
        assert!(optimizer.is_large_organization(1000));
        assert!(optimizer.is_large_organization(5000));
    }

    #[test]
    pub(super) fn test_sort_direction_default() {
        let direction = SortDirection::default();
        assert_eq!(direction, SortDirection::Asc);
    }

    #[tokio::test]
    async fn test_cache_member_list() {
        let optimizer = LargeOrgOptimizer::default();
        let org_id = Uuid::new_v4();
        let member_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];

        optimizer
            .cache_member_list(org_id, member_ids.clone(), 3, false, vec![0])
            .await;

        let cached = optimizer.get_cached_members(org_id).await;
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.member_ids.len(), 3);
        assert_eq!(cached.total_count, 3);
        assert!(!cached.is_partial);
    }

    #[tokio::test]
    async fn test_cache_permissions() {
        let optimizer = LargeOrgOptimizer::default();
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        optimizer
            .cache_permissions(
                org_id,
                user_id,
                "bot",
                vec!["read".to_string(), "write".to_string()],
            )
            .await;

        let cached = optimizer
            .get_cached_permissions(org_id, user_id, "bot")
            .await;
        assert!(cached.is_some());
        let perms = cached.unwrap();
        assert_eq!(perms.len(), 2);
        assert!(perms.contains(&"read".to_string()));
        assert!(perms.contains(&"write".to_string()));
    }

    #[tokio::test]
    async fn test_invalidate_member_cache() {
        let optimizer = LargeOrgOptimizer::default();
        let org_id = Uuid::new_v4();

        optimizer
            .cache_member_list(org_id, vec![Uuid::new_v4()], 1, false, vec![0])
            .await;

        optimizer.invalidate_member_cache(org_id).await;

        let cached = optimizer.get_cached_members(org_id).await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_invalidate_permission_cache_all() {
        let optimizer = LargeOrgOptimizer::default();
        let org_id = Uuid::new_v4();
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();

        optimizer
            .cache_permissions(org_id, user_id1, "bot", vec!["read".to_string()])
            .await;
        optimizer
            .cache_permissions(org_id, user_id2, "bot", vec!["write".to_string()])
            .await;

        optimizer.invalidate_permission_cache(org_id, None).await;

        assert!(optimizer
            .get_cached_permissions(org_id, user_id1, "bot")
            .await
            .is_none());
        assert!(optimizer
            .get_cached_permissions(org_id, user_id2, "bot")
            .await
            .is_none());
    }

    #[tokio::test]
    async fn test_invalidate_permission_cache_specific_user() {
        let optimizer = LargeOrgOptimizer::default();
        let org_id = Uuid::new_v4();
        let user_id1 = Uuid::new_v4();
        let user_id2 = Uuid::new_v4();

        optimizer
            .cache_permissions(org_id, user_id1, "bot", vec!["read".to_string()])
            .await;
        optimizer
            .cache_permissions(org_id, user_id2, "bot", vec!["write".to_string()])
            .await;

        optimizer
            .invalidate_permission_cache(org_id, Some(user_id1))
            .await;

        assert!(optimizer
            .get_cached_permissions(org_id, user_id1, "bot")
            .await
            .is_none());
        assert!(optimizer
            .get_cached_permissions(org_id, user_id2, "bot")
            .await
            .is_some());
    }

    #[tokio::test]
    async fn test_cache_hit_rate() {
        let optimizer = LargeOrgOptimizer::default();

        let rate = optimizer.get_cache_hit_rate().await;
        assert_eq!(rate, 0.0);
    }

    #[tokio::test]
    async fn test_generate_optimization_report() {
        let optimizer = LargeOrgOptimizer::default();
        let org_id = Uuid::new_v4();

        let report = optimizer.generate_optimization_report(org_id, 500).await;
        assert_eq!(report.organization_id, org_id);
        assert_eq!(report.member_count, 500);
        assert!(!report.is_large_org);

        let large_report = optimizer.generate_optimization_report(org_id, 2000).await;
        assert!(large_report.is_large_org);
    }

    #[tokio::test]
    async fn test_cleanup_expired_caches() {
        let optimizer = LargeOrgOptimizer::default();
        let org_id = Uuid::new_v4();

        optimizer
            .cache_member_list(org_id, vec![Uuid::new_v4()], 1, false, vec![0])
            .await;

        let result = optimizer.cleanup_expired_caches().await;
        assert_eq!(result.members_removed, 0);
        assert_eq!(result.permissions_removed, 0);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let optimizer = LargeOrgOptimizer::default();

        let stats = optimizer.get_statistics().await;
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.slow_queries, 0);
    }

    #[tokio::test]
    async fn test_partition_manager() {
        let manager = PartitionManager::new(PartitionConfig::default());
        let org_id = Uuid::new_v4();

        let partition = manager
            .create_partition(org_id, "user_id")
            .await;
        assert_eq!(partition.partition_key, "user_id");
        assert_eq!(partition.row_count, 0);

        let partitions = manager.get_partitions(org_id).await;
        assert_eq!(partitions.len(), 1);
    }

    #[tokio::test]
    async fn test_partition_split_merge_thresholds() {
        let manager = PartitionManager::new(PartitionConfig::default());
        let org_id = Uuid::new_v4();

        let partition = manager.create_partition(org_id, "id").await;

        assert!(!manager.should_split(&partition).await);
        assert!(manager.should_merge(&partition).await);

        let mut large_partition = partition.clone();
        large_partition.row_count = 9000;
        assert!(manager.should_split(&large_partition).await);
        assert!(!manager.should_merge(&large_partition).await);
    }

    #[tokio::test]
    async fn test_paginated_result() {
        let result: PaginatedResult<Uuid> = PaginatedResult {
            items: vec![Uuid::new_v4(), Uuid::new_v4()],
            total_count: 100,
            page: 0,
            page_size: 10,
            total_pages: 10,
            has_next: true,
            has_previous: false,
        };

        assert_eq!(result.items.len(), 2);
        assert!(result.has_next);
        assert!(!result.has_previous);
    }

    #[test]
    pub(super) fn test_large_org_error_display() {
        let errors = vec![
            (LargeOrgError::DatabaseError("test".to_string()), "Database error: test"),
            (LargeOrgError::CacheError("test".to_string()), "Cache error: test"),
            (LargeOrgError::PartitionError("test".to_string()), "Partition error: test"),
            (LargeOrgError::InvalidQuery("test".to_string()), "Invalid query: test"),
        ];

        for (error, expected) in errors {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    pub(super) fn test_recommendation_categories() {
        let categories = vec![
            RecommendationCategory::Caching,
            RecommendationCategory::Indexing,
            RecommendationCategory::Partitioning,
            RecommendationCategory::QueryOptimization,
            RecommendationCategory::DataStructure,
            RecommendationCategory::BatchProcessing,
        ];

        for category in categories {
            let serialized = serde_json::to_string(&category).unwrap();
            let deserialized: RecommendationCategory = serde_json::from_str(&serialized).unwrap();
            assert_eq!(category, deserialized);
        }
    }

    #[test]
    pub(super) fn test_impact_levels() {
        let impacts = vec![Impact::High, Impact::Medium, Impact::Low];

        for impact in impacts {
            let serialized = serde_json::to_string(&impact).unwrap();
            let deserialized: Impact = serde_json::from_str(&serialized).unwrap();
            assert_eq!(impact, deserialized);
        }
    }
