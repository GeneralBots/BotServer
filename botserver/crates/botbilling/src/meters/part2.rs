use super::*;

impl UsageMeteringService {
    pub(super) fn calculate_trend(
        &self,
        storage: &MeteringStorage,
        organization_id: Uuid,
        metric: UsageMetric,
    ) -> UsageTrend {
        let snapshots = match storage.daily_snapshots.get(&organization_id) {
            Some(s) if s.len() >= 2 => s,
            _ => return UsageTrend::Unknown,
        };

        let recent: Vec<u64> = snapshots
            .iter()
            .rev()
            .take(7)
            .filter_map(|s| s.metrics.get(&metric).copied())
            .collect();

        if recent.len() < 2 {
            return UsageTrend::Unknown;
        }

        let first_half_avg: f64 =
            recent[recent.len() / 2..].iter().sum::<u64>() as f64 / (recent.len() / 2) as f64;
        let second_half_avg: f64 =
            recent[..recent.len() / 2].iter().sum::<u64>() as f64 / (recent.len() / 2) as f64;

        let change_percent = if first_half_avg > 0.0 {
            ((second_half_avg - first_half_avg) / first_half_avg) * 100.0
        } else {
            0.0
        };

        if change_percent > 10.0 {
            UsageTrend::Increasing
        } else if change_percent < -10.0 {
            UsageTrend::Decreasing
        } else {
            UsageTrend::Stable
        }
    }

    pub fn aggregation_interval(&self) -> u64 {
        self.aggregation_interval_secs
    }
}
