use super::*;

impl<K, V> PerformanceCache<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(region: CacheRegion, config: CacheConfig) -> Self {
        let cache = Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            config: config.clone(),
            statistics: Arc::new(CacheStatistics::default()),
            region,
        };

        let entries = cache.entries.clone();
        let stats = cache.statistics.clone();
        let cleanup_interval = config.cleanup_interval_seconds;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(cleanup_interval)
            );
            loop {
                interval.tick().await;
                let mut map = entries.write().await;
                let before = map.len();
                map.retain(|_, entry| {
                    if entry.is_expired() {
                        stats.record_expiration();
                        false
                    } else {
                        true
                    }
                });
                let removed = before - map.len();
                if removed > 0 {
                    stats.entry_count.fetch_sub(removed as u64, Ordering::Relaxed);
                }
            }
        });

        cache
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.get_mut(key) {
            if entry.is_expired() {
                entries.remove(key);
                self.statistics.record_expiration();
                self.statistics.entry_count.fetch_sub(1, Ordering::Relaxed);
                self.statistics.record_miss();
                return None;
            }

            entry.touch();
            self.statistics.record_hit();
            return Some(entry.value.clone());
        }

        self.statistics.record_miss();
        None
    }

    pub async fn set(&self, key: K, value: V, size_bytes: usize) {
        self.set_with_ttl(key, value, self.config.default_ttl_seconds, size_bytes).await;
    }

    pub async fn set_with_ttl(&self, key: K, value: V, ttl_seconds: u64, size_bytes: usize) {
        let mut entries = self.entries.write().await;

        if entries.len() >= self.config.max_entries {
            self.evict_entries(&mut entries).await;
        }

        let entry = CacheEntry::new(value, ttl_seconds, size_bytes);
        let is_new = !entries.contains_key(&key);

        entries.insert(key, entry);

        if is_new {
            self.statistics.entry_count.fetch_add(1, Ordering::Relaxed);
        }
        self.statistics.total_size_bytes.fetch_add(size_bytes as u64, Ordering::Relaxed);
    }

    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.remove(key) {
            self.statistics.entry_count.fetch_sub(1, Ordering::Relaxed);
            self.statistics.total_size_bytes.fetch_sub(entry.size_bytes as u64, Ordering::Relaxed);
            return Some(entry.value);
        }

        None
    }

    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        entries.clear();
        self.statistics.entry_count.store(0, Ordering::Relaxed);
        self.statistics.total_size_bytes.store(0, Ordering::Relaxed);
    }

    pub async fn contains(&self, key: &K) -> bool {
        let entries = self.entries.read().await;

        if let Some(entry) = entries.get(key) {
            return !entry.is_expired();
        }

        false
    }

    pub async fn size(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    pub fn statistics(&self) -> CacheStatisticsSummary {
        self.statistics.to_summary()
    }

    async fn evict_entries(&self, entries: &mut HashMap<K, CacheEntry<V>>) {
        let region_config = self.config.region_configs.get(&self.region);
        let policy = region_config
            .map(|c| c.eviction_policy)
            .unwrap_or(EvictionPolicy::Lru);

        let entries_to_remove = entries.len() / 10 + 1;

        let keys_to_remove: Vec<K> = match policy {
            EvictionPolicy::Lru => {
                let mut sorted: Vec<_> = entries.iter().collect();
                sorted.sort_by_key(|(_, e)| e.last_accessed);
                sorted.into_iter().take(entries_to_remove).map(|(k, _)| k.clone()).collect()
            }
            EvictionPolicy::Lfu => {
                let mut sorted: Vec<_> = entries.iter().collect();
                sorted.sort_by_key(|(_, e)| e.access_count);
                sorted.into_iter().take(entries_to_remove).map(|(k, _)| k.clone()).collect()
            }
            EvictionPolicy::Fifo => {
                let mut sorted: Vec<_> = entries.iter().collect();
                sorted.sort_by_key(|(_, e)| e.created_at);
                sorted.into_iter().take(entries_to_remove).map(|(k, _)| k.clone()).collect()
            }
            EvictionPolicy::Ttl => {
                let mut sorted: Vec<_> = entries.iter().collect();
                sorted.sort_by_key(|(_, e)| e.expires_at);
                sorted.into_iter().take(entries_to_remove).map(|(k, _)| k.clone()).collect()
            }
            EvictionPolicy::Random => {
                entries.keys().take(entries_to_remove).cloned().collect()
            }
        };

        for key in keys_to_remove {
            if let Some(entry) = entries.remove(&key) {
                self.statistics.record_eviction();
                self.statistics.entry_count.fetch_sub(1, Ordering::Relaxed);
                self.statistics.total_size_bytes.fetch_sub(entry.size_bytes as u64, Ordering::Relaxed);
            }
        }
    }

    pub async fn get_or_insert<F, Fut>(&self, key: K, loader: F, size_bytes: usize) -> V
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = V>,
    {
        if let Some(value) = self.get(&key).await {
            return value;
        }

        let value = loader().await;
        self.set(key, value.clone(), size_bytes).await;
        value
    }
}
