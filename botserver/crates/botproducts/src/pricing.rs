use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::inventory::Product;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PricingStrategy {
    Fixed,
    CostPlus,
    Competitive,
    Tiered,
    Dynamic,
    Promotional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceList {
    pub id: Uuid,
    pub name: String,
    pub currency: String,
    pub active: bool,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRule {
    pub id: Uuid,
    pub price_list_id: Uuid,
    pub product_id: Option<Uuid>,
    pub category: Option<String>,
    pub strategy: PricingStrategy,
    pub markup_percent: Option<f64>,
    pub fixed_price_cents: Option<i64>,
    pub min_price_cents: Option<i64>,
    pub max_price_cents: Option<i64>,
    pub min_quantity: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Promotion {
    pub id: Uuid,
    pub name: String,
    pub code: Option<String>,
    pub discount_type: DiscountType,
    pub discount_value: f64,
    pub applies_to: PromotionTarget,
    pub min_purchase_cents: Option<i64>,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub max_uses: Option<i32>,
    pub current_uses: i32,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscountType {
    Percent,
    FixedAmount,
    BuyXGetY,
    FreeShipping,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromotionTarget {
    AllProducts,
    Category(String),
    Product(Uuid),
    Brand(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceQuote {
    pub product_id: Uuid,
    pub list_price_cents: i64,
    pub final_price_cents: i64,
    pub discount_cents: i64,
    pub applied_rule_id: Option<Uuid>,
    pub applied_promotion_id: Option<Uuid>,
    pub tax_cents: i64,
    pub total_cents: i64,
}

pub struct PricingEngine;

impl PricingEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn quote(
        &self,
        product: &Product,
        rules: &[PriceRule],
        promotions: &[Promotion],
        quantity: i32,
        now: DateTime<Utc>,
    ) -> PriceQuote {
        let list_price = self.base_price(product, rules, quantity);
        let promotion_price = self.apply_promotions(list_price, product, promotions, quantity, now);
        let final_price = promotion_price.max(product.cost_cents);
        let discount = list_price - final_price;
        let tax_cents = ((final_price as f64) * product.tax_rate) as i64;
        PriceQuote {
            product_id: product.id,
            list_price_cents: list_price,
            final_price_cents: final_price,
            discount_cents: discount,
            applied_rule_id: None,
            applied_promotion_id: None,
            tax_cents,
            total_cents: final_price + tax_cents,
        }
    }

    fn base_price(&self, product: &Product, rules: &[PriceRule], quantity: i32) -> i64 {
        let mut candidates: Vec<i64> = vec![product.price_cents];
        for rule in rules.iter().filter(|r| r.active) {
            if let Some(pid) = rule.product_id {
                if pid != product.id {
                    continue;
                }
            }
            if let Some(cat) = &rule.category {
                if product.category.as_deref() != Some(cat.as_str()) {
                    continue;
                }
            }
            if quantity < rule.min_quantity {
                continue;
            }
            let price = match rule.strategy {
                PricingStrategy::Fixed => rule.fixed_price_cents.unwrap_or(product.price_cents),
                PricingStrategy::CostPlus => {
                    let markup = rule.markup_percent.unwrap_or(0.0);
                    product.cost_cents + ((product.cost_cents as f64) * markup / 100.0) as i64
                }
                PricingStrategy::Tiered => {
                    if quantity >= 100 {
                        rule.fixed_price_cents.unwrap_or(product.price_cents) - 100
                    } else if quantity >= 10 {
                        rule.fixed_price_cents.unwrap_or(product.price_cents) - 50
                    } else {
                        rule.fixed_price_cents.unwrap_or(product.price_cents)
                    }
                }
                PricingStrategy::Competitive
                | PricingStrategy::Dynamic
                | PricingStrategy::Promotional => {
                    rule.fixed_price_cents.unwrap_or(product.price_cents)
                }
            };
            candidates.push(price);
        }
        candidates
            .into_iter()
            .min()
            .unwrap_or(product.price_cents)
            .max(0)
    }

    fn apply_promotions(
        &self,
        price: i64,
        product: &Product,
        promotions: &[Promotion],
        quantity: i32,
        now: DateTime<Utc>,
    ) -> i64 {
        let mut best = price;
        for promo in promotions.iter().filter(|p| p.active) {
            if now < promo.valid_from || now > promo.valid_until {
                continue;
            }
            if let Some(max) = promo.max_uses {
                if promo.current_uses >= max {
                    continue;
                }
            }
            if let Some(min) = promo.min_purchase_cents {
                if price < min {
                    continue;
                }
            }
            let matches_target = match &promo.applies_to {
                PromotionTarget::AllProducts => true,
                PromotionTarget::Category(c) => product.category.as_deref() == Some(c.as_str()),
                PromotionTarget::Product(pid) => *pid == product.id,
                PromotionTarget::Brand(b) => product.description.as_deref().map(|d| d.contains(b)).unwrap_or(false),
            };
            if !matches_target {
                continue;
            }
            let new_price = match promo.discount_type {
                DiscountType::Percent => {
                    let discount = ((price as f64) * promo.discount_value / 100.0) as i64;
                    price - discount
                }
                DiscountType::FixedAmount => price - (promo.discount_value as i64),
                DiscountType::BuyXGetY => {
                    if quantity >= (promo.discount_value as i32) {
                        price / 2
                    } else {
                        price
                    }
                }
                DiscountType::FreeShipping => price,
            };
            if new_price < best {
                best = new_price;
            }
        }
        best.max(0)
    }
}

impl Default for PricingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn product(price: i64, cost: i64) -> Product {
        Product {
            id: Uuid::new_v4(),
            sku: "SKU-1".into(),
            name: "Test".into(),
            description: None,
            category: Some("food".into()),
            barcode: None,
            unit: "un".into(),
            weight_grams: None,
            cost_cents: cost,
            price_cents: price,
            tax_rate: 0.0,
            active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn base_quote_uses_list_price() {
        let engine = PricingEngine::new();
        let p = product(1000, 600);
        let q = engine.quote(&p, &[], &[], 1, Utc::now());
        assert_eq!(q.list_price_cents, 1000);
        assert_eq!(q.final_price_cents, 1000);
    }
}
