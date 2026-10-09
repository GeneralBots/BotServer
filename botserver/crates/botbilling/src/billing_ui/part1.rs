use super::*;

pub(super) fn bd_to_f64(bd: &BigDecimal) -> f64 {
    bd.to_f64().unwrap_or(0.0)
}
