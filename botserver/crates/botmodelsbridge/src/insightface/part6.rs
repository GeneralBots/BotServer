use super::*;

impl Default for ClusteringConfig {
    fn default() -> Self {
        Self {
            min_cluster_size: 2,
            distance_threshold: 0.6,
            algorithm: ClusteringAlgorithm::DbScan,
        }
    }
}
