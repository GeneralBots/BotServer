use super::*;

impl AlertCondition {
    pub fn evaluate(&self, current: f64, previous: Option<f64>, absent_seconds: u64) -> bool {
        match self {
            Self::GreaterThan(threshold) => current > *threshold,
            Self::LessThan(threshold) => current < *threshold,
            Self::Equals(threshold) => (current - threshold).abs() < f64::EPSILON,
            Self::NotEquals(threshold) => (current - threshold).abs() >= f64::EPSILON,
            Self::GreaterThanOrEqual(threshold) => current >= *threshold,
            Self::LessThanOrEqual(threshold) => current <= *threshold,
            Self::AbsentFor(seconds) => absent_seconds >= *seconds,
            Self::RateOfChange {
                threshold,
                window_seconds: _,
            } => {
                if let Some(prev) = previous {
                    let rate = (current - prev).abs();
                    rate > *threshold
                } else {
                    false
                }
            }
        }
    }
}
