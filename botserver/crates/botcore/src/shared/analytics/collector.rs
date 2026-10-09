use super::*;

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            aggregates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record(&self, name: String, value: f64, labels: HashMap<String, String>) {
        let metric = Metric {
            name: name.clone(),
            value,
            labels,
            timestamp: Utc::now(),
        };

        let mut metrics = self.metrics.write().await;
        metrics.push(metric);

        let mut aggregates = self.aggregates.write().await;
        let entry = aggregates.entry(name).or_insert(0.0);
        *entry += value;

        if metrics.len() > 10000 {
            let cutoff = Utc::now() - Duration::hours(1);
            metrics.retain(|m| m.timestamp > cutoff);
        }
    }

    pub async fn increment(&self, name: String, labels: HashMap<String, String>) {
        self.record(name, 1.0, labels).await;
    }

    pub async fn gauge(&self, name: String, value: f64, labels: HashMap<String, String>) {
        self.record(name, value, labels).await;
    }

    pub async fn get_metrics(&self) -> Vec<Metric> {
        self.metrics.read().await.clone()
    }

    pub async fn get_aggregate(&self, name: &str) -> Option<f64> {
        self.aggregates.read().await.get(name).copied()
    }

    pub async fn get_rate(&self, name: &str, window: Duration) -> f64 {
        let cutoff = Utc::now() - window;
        let metrics = self.metrics.read().await;
        let count = metrics
            .iter()
            .filter(|m| m.name == name && m.timestamp > cutoff)
            .count();
        count as f64 / window.num_seconds() as f64
    }

    pub async fn get_percentile(&self, name: &str, percentile: f64) -> Option<f64> {
        let metrics = self.metrics.read().await;
        let mut values: Vec<f64> = metrics
            .iter()
            .filter(|m| m.name == name)
            .map(|m| m.value)
            .collect();

        if values.is_empty() {
            return None;
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let index = ((percentile / 100.0) * values.len() as f64) as usize;
        values.get(index.min(values.len() - 1)).copied()
    }
}
