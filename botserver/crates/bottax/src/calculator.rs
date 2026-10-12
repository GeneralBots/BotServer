//! Service-tax calculation engine (issue #722).
//!
//! Brazilian service-revenue taxation, computed from a composite percentage
//! per tax (the same semantics used by `billing_tax_rates.rate`, which is a
//! percent value applied to the gross amount):
//!
//!   * IRPJ       — default 4.80%  (32% presumed-profit base x 15%)
//!   * CSLL       — default 2.88%  (32% base x 9%)
//!   * PIS/COFINS — default 3.65%
//!   * ISS        — default 5.00%
//!
//! Rates are never hardcoded in bot logic: they are loaded dynamically from
//! `billing_tax_rates` (existing fiscal model, branch-scoped) or per-bot
//! `bot_configuration` keys (`tax-irpj`, `tax-csll`, `tax-pis-cofins`,
//! `tax-iss`), falling back to the defaults above only when no source exists.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Composite rates expressed as percentages (e.g. 4.80 = 4.80%).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxRates {
    pub irpj_pct: Decimal,
    pub csll_pct: Decimal,
    pub pis_cofins_pct: Decimal,
    pub iss_pct: Decimal,
}

impl Default for TaxRates {
    fn default() -> Self {
        Self {
            irpj_pct: Decimal::new(480, 2),
            csll_pct: Decimal::new(288, 2),
            pis_cofins_pct: Decimal::new(365, 2),
            iss_pct: Decimal::new(500, 2),
        }
    }
}

impl TaxRates {
    pub fn rate_names() -> [&'static str; 4] {
        ["IRPJ", "CSLL", "PIS/COFINS", "ISS"]
    }

    pub fn set(&mut self, name: &str, pct: Decimal) {
        match name.trim().to_uppercase().as_str() {
            "IRPJ" => self.irpj_pct = pct,
            "CSLL" => self.csll_pct = pct,
            "PIS/COFINS" | "PIS_COFINS" | "PIS" => self.pis_cofins_pct = pct,
            "ISS" => self.iss_pct = pct,
            _ => {}
        }
    }
}

/// Per-tax breakdown for a single calculation.
#[derive(Debug, Clone, Serialize)]
pub struct TaxBreakdown {
    pub service_value: Decimal,
    pub irpj: Decimal,
    pub csll: Decimal,
    pub pis_cofins: Decimal,
    pub iss: Decimal,
    pub total_taxes: Decimal,
    pub effective_rate: Decimal,
}

/// Computes the tax breakdown for a service revenue value.
pub fn calculate_service_tax(service_value: Decimal, rates: &TaxRates) -> TaxBreakdown {
    let pct = |v: &Decimal| service_value * *v / Decimal::new(100, 0);
    let irpj = pct(&rates.irpj_pct).round_dp(2);
    let csll = pct(&rates.csll_pct).round_dp(2);
    let pis_cofins = pct(&rates.pis_cofins_pct).round_dp(2);
    let iss = pct(&rates.iss_pct).round_dp(2);
    let total_taxes = irpj + csll + pis_cofins + iss;
    let effective_rate = if service_value.is_zero() {
        Decimal::ZERO
    } else {
        (total_taxes * Decimal::new(100, 0) / service_value).round_dp(2)
    };
    TaxBreakdown {
        service_value,
        irpj,
        csll,
        pis_cofins,
        iss,
        total_taxes,
        effective_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_service_tax_default_rates() {
        let rates = TaxRates::default();
        let b = calculate_service_tax(Decimal::new(10000, 0), &rates);
        assert_eq!(b.irpj, Decimal::new(48000, 2));
        assert_eq!(b.csll, Decimal::new(28800, 2));
        assert_eq!(b.pis_cofins, Decimal::new(36500, 2));
        assert_eq!(b.iss, Decimal::new(50000, 2));
        assert_eq!(b.total_taxes, Decimal::new(163300, 2));
        assert_eq!(b.effective_rate, Decimal::new(1633, 2));
    }

    #[test]
    fn test_calculate_zero_value() {
        let b = calculate_service_tax(Decimal::ZERO, &TaxRates::default());
        assert_eq!(b.total_taxes, Decimal::ZERO);
        assert_eq!(b.effective_rate, Decimal::ZERO);
    }

    #[test]
    fn test_dynamic_rate_override() {
        // Rates are percentages: 2.00 ISS and 1.50 IRPJ, which is how the
        // defaults are expressed (Decimal::new(500, 2) == 5.00%).
        let mut rates = TaxRates::default();
        rates.set("ISS", Decimal::new(200, 2));
        rates.set("IRPJ", Decimal::new(150, 2));
        assert_eq!(rates.iss_pct, Decimal::new(200, 2));
        assert_eq!(rates.irpj_pct, Decimal::new(150, 2));

        let b = calculate_service_tax(Decimal::new(10000, 0), &rates);
        assert_eq!(b.iss, Decimal::new(20000, 2));
        assert_eq!(b.irpj, Decimal::new(15000, 2));
    }

    #[test]
    fn test_rate_names_are_stable() {
        assert_eq!(TaxRates::rate_names(), ["IRPJ", "CSLL", "PIS/COFINS", "ISS"]);
    }
}
