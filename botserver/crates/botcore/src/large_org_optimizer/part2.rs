use super::*;

impl LargeOrgOptimizer {
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            config,
            member_cache: Arc::new(RwLock::new(HashMap::new())),
            permission_cache: Arc::new(RwLock::new(HashMap::new())),
            query_stats: Arc::new(RwLock::new(QueryStatistics::default())),
            partition_manager: Arc::new(PartitionManager::new(PartitionConfig::default())),
        }
    }

    pub fn is_large_organization(&self, member_count: usize) -> bool {
        member_count >= self.config.large_org_threshold
    }

    pub async fn get_members_paginated(
        &self,
        query: &PaginatedQuery,
    ) -> Result<PaginatedResult<Uuid>, LargeOrgError> {
        let start = std::time::Instant::now();

        if let Some(cached) = self.get_cached_members(query.organization_id).await {
            if !cached.is_partial || cached.loaded_pages.contains(&query.page) {
                self.record_cache_hit().await;
                let result = self.paginate_cached_members(&cached, query);
                self.record_query_time(start.elapsed().as_millis() as u64).await;
                return Ok(result);
            }
        }

        self.record_cache_miss().await;

        let members = self.fetch_members_page(query).await?;
        self.record_query_time(start.elapsed().as_millis() as u64).await;

        Ok(members)
    }

    pub(super) async fn get_cached_members(&self, organization_id: Uuid) -> Option<CachedMemberList> {
        let cache = self.member_cache.read().await;
        cache.get(&organization_id).and_then(|cached| {
            if Utc::now() < cached.expires_at {
                Some(cached.clone())
            } else {
                None
            }
        })
    }

    pub(super) fn paginate_cached_members(
        &self,
        cached: &CachedMemberList,
        query: &PaginatedQuery,
    ) -> PaginatedResult<Uuid> {
        let start_idx = query.page * query.page_size;
        let end_idx = (start_idx + query.page_size).min(cached.member_ids.len());

        let items = if start_idx < cached.member_ids.len() {
            cached.member_ids[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };

        let total_pages = cached.total_count.div_ceil(query.page_size);

        PaginatedResult {
            items,
            total_count: cached.total_count,
            page: query.page,
            page_size: query.page_size,
            total_pages,
            has_next: query.page + 1 < total_pages,
            has_previous: query.page > 0,
        }
    }

    async fn fetch_members_page(
        &self,
        query: &PaginatedQuery,
    ) -> Result<PaginatedResult<Uuid>, LargeOrgError> {
        let items = Vec::new();
        let total_count: usize = 0;

        let total_pages = if total_count > 0 {
            total_count.div_ceil(query.page_size)
        } else {
            0
        };

        Ok(PaginatedResult {
            items,
            total_count,
            page: query.page,
            page_size: query.page_size,
            total_pages,
            has_next: query.page + 1 < total_pages,
            has_previous: query.page > 0,
        })
    }

    pub async fn cache_member_list(
        &self,
        organization_id: Uuid,
        member_ids: Vec<Uuid>,
        total_count: usize,
        is_partial: bool,
        pages: Vec<usize>,
    ) {
        let now = Utc::now();
        let cached = CachedMemberList {
            member_ids,
            total_count,
            expires_at: now + chrono::Duration::seconds(self.config.member_cache_ttl_seconds as i64),
            is_partial,
            loaded_pages: pages,
        };

        let mut cache = self.member_cache.write().await;
        cache.insert(organization_id, cached);
    }

    pub async fn get_cached_permissions(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        resource_type: &str,
    ) -> Option<Vec<String>> {
        let key = PermissionCacheKey {
            organization_id,
            user_id,
            resource_type: resource_type.to_string(),
        };

        let cache = self.permission_cache.read().await;
        cache.get(&key).and_then(|cached| {
            if Utc::now() < cached.expires_at {
                Some(cached.permissions.clone())
            } else {
                None
            }
        })
    }

    pub async fn cache_permissions(
        &self,
        organization_id: Uuid,
        user_id: Uuid,
        resource_type: &str,
        permissions: Vec<String>,
    ) {
        let key = PermissionCacheKey {
            organization_id,
            user_id,
            resource_type: resource_type.to_string(),
        };

        let now = Utc::now();
        let cached = CachedPermissions {
            permissions,
            expires_at: now + chrono::Duration::seconds(self.config.permission_cache_ttl_seconds as i64),
        };

        let mut cache = self.permission_cache.write().await;
        cache.insert(key, cached);
    }

    pub async fn invalidate_member_cache(&self, organization_id: Uuid) {
        let mut cache = self.member_cache.write().await;
        cache.remove(&organization_id);
    }

    pub async fn invalidate_permission_cache(&self, organization_id: Uuid, user_id: Option<Uuid>) {
        let mut cache = self.permission_cache.write().await;

        if let Some(uid) = user_id {
            cache.retain(|k, _| !(k.organization_id == organization_id && k.user_id == uid));
        } else {
            cache.retain(|k, _| k.organization_id != organization_id);
        }
    }

    pub async fn invalidate_all_caches(&self, organization_id: Uuid) {
        self.invalidate_member_cache(organization_id).await;
        self.invalidate_permission_cache(organization_id, None).await;
    }

    async fn record_cache_hit(&self) {
        let stats = self.query_stats.read().await;
        stats.cache_hits.fetch_add(1, Ordering::Relaxed);
        stats.total_queries.fetch_add(1, Ordering::Relaxed);
    }

    async fn record_cache_miss(&self) {
        let stats = self.query_stats.read().await;
        stats.cache_misses.fetch_add(1, Ordering::Relaxed);
        stats.total_queries.fetch_add(1, Ordering::Relaxed);
    }

    async fn record_query_time(&self, time_ms: u64) {
        let stats = self.query_stats.read().await;
let total = stats.total_queries.load(Ordering::Relaxed);
let current_avg = stats.avg_query_time_ms.load(Ordering::Relaxed);

let new_avg = ((current_avg * (total.saturating_sub(1))) + time_ms).checked_div(total).unwrap_or(0);
stats.avg_query_time_ms.store(new_avg, Ordering::Relaxed);

        if time_ms > 1000 {
            stats.slow_queries.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub async fn get_cache_hit_rate(&self) -> f64 {
        let stats = self.query_stats.read().await;
        let hits = stats.cache_hits.load(Ordering::Relaxed) as f64;
        let total = stats.total_queries.load(Ordering::Relaxed) as f64;

        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }

    pub async fn generate_optimization_report(
        &self,
        organization_id: Uuid,
        member_count: usize,
    ) -> OptimizationReport {
        let is_large_org = self.is_large_organization(member_count);
        let cache_hit_rate = self.get_cache_hit_rate().await;

        let stats = self.query_stats.read().await;
        let avg_query_time_ms = stats.avg_query_time_ms.load(Ordering::Relaxed) as f64;
        drop(stats);

        let recommendations = self.generate_recommendations(
            member_count,
            is_large_org,
            cache_hit_rate,
            avg_query_time_ms,
        );

        OptimizationReport {
            organization_id,
            member_count,
            is_large_org,
            cache_hit_rate,
            avg_query_time_ms,
            recommendations,
            generated_at: Utc::now(),
        }
    }

    pub(super) fn generate_recommendations(
        &self,
        member_count: usize,
        is_large_org: bool,
        cache_hit_rate: f64,
        avg_query_time_ms: f64,
    ) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        if cache_hit_rate < 0.5 {
            recommendations.push(OptimizationRecommendation {
                category: RecommendationCategory::Caching,
                description: "Cache hit rate is below 50%. Consider increasing cache TTL or preloading frequently accessed data.".to_string(),
                impact: Impact::High,
                implementation: "Increase member_cache_ttl_seconds and permission_cache_ttl_seconds in optimizer config".to_string(),
            });
        }

        if avg_query_time_ms > 500.0 {
            recommendations.push(OptimizationRecommendation {
                category: RecommendationCategory::QueryOptimization,
                description: "Average query time exceeds 500ms. Consider adding indexes or optimizing queries.".to_string(),
                impact: Impact::High,
                implementation: "Review slow query logs and add appropriate database indexes".to_string(),
            });
        }

        if is_large_org && !self.config.enable_partitioning {
            recommendations.push(OptimizationRecommendation {
                category: RecommendationCategory::Partitioning,
                description: "Large organization detected without partitioning enabled.".to_string(),
                impact: Impact::Medium,
                implementation: "Enable partitioning in optimizer config for better query performance".to_string(),
            });
        }

        if member_count > 5000 && self.config.batch_size < 200 {
            recommendations.push(OptimizationRecommendation {
                category: RecommendationCategory::BatchProcessing,
                description: "Very large organization would benefit from larger batch sizes.".to_string(),
                impact: Impact::Medium,
                implementation: "Increase batch_size in optimizer config to 200 or higher".to_string(),
            });
        }

        if is_large_org && !self.config.enable_lazy_loading {
            recommendations.push(OptimizationRecommendation {
                category: RecommendationCategory::DataStructure,
                description: "Lazy loading disabled for large organization.".to_string(),
                impact: Impact::Medium,
                implementation: "Enable lazy_loading to reduce initial load times".to_string(),
            });
        }

        recommendations
    }

    pub async fn batch_process<T, F, Fut>(
        &self,
        items: Vec<T>,
        processor: F,
    ) -> Vec<Result<(), LargeOrgError>>
    where
        T: Send + Sync + Clone + 'static,
        F: Fn(Vec<T>) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), LargeOrgError>> + Send,
    {
        let batch_size = self.config.batch_size;
        let mut results = Vec::new();

        for chunk in items.chunks(batch_size) {
            let result = processor(chunk.to_vec()).await;
            results.push(result);
        }

        results
    }

    pub async fn cleanup_expired_caches(&self) -> CleanupResult {
        let now = Utc::now();

        let members_removed = {
            let mut member_cache = self.member_cache.write().await;
            let original_len = member_cache.len();
            member_cache.retain(|_, v| v.expires_at > now);
            original_len - member_cache.len()
        };

        let permissions_removed = {
            let mut permission_cache = self.permission_cache.write().await;
            let original_len = permission_cache.len();
            permission_cache.retain(|_, v| v.expires_at > now);
            original_len - permission_cache.len()
        };

        CleanupResult {
            members_removed,
            permissions_removed,
            cleaned_at: now,
        }
    }

    pub fn config(&self) -> &OptimizerConfig {
        &self.config
    }

    pub fn partition_manager(&self) -> &Arc<PartitionManager> {
        &self.partition_manager
    }

    pub async fn get_statistics(&self) -> OptimizerStatistics {
        let stats = self.query_stats.read().await;
        let member_cache = self.member_cache.read().await;
        let permission_cache = self.permission_cache.read().await;

        OptimizerStatistics {
            total_queries: stats.total_queries.load(Ordering::Relaxed),
            cache_hits: stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: stats.cache_misses.load(Ordering::Relaxed),
            slow_queries: stats.slow_queries.load(Ordering::Relaxed),
            avg_query_time_ms: stats.avg_query_time_ms.load(Ordering::Relaxed),
            member_cache_size: member_cache.len(),
            permission_cache_size: permission_cache.len(),
        }
    }
}
