//! Canonical request payloads for the banking BASIC keywords.
//!
//! `IMPORT BANK STATEMENT`, `IMPORT PLATFORM ORDERS`, `RECONCILE`,
//! `GET UNMATCHED DELIVERIES`, `ADD RECONCILE RULE`, `DELIVERY MARGINS` and
//! `REPORT REVENUE BY PLATFORM` only bind arguments; the CSV parsing, the
//! matching and the aggregations they report belong to this crate (#1506).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

mod import;

/// The `kind` discriminator of the command the runtime routes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BankingRequestKind {
    BankTransactions,
    DeliveryTransactions,
    UnmatchedQuery,
    ReconciliationRule,
    RevenueByPlatform,
}

/// A banking command or computed report, ready to be dispatched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BankingRequest {
    BankTransactions(BankTransactionImport),
    DeliveryTransactions(DeliveryTransactionImport),
    Reconciliation(ReconciliationSummary),
    UnmatchedQuery(UnmatchedDeliveriesQuery),
    ReconciliationRule(ReconciliationRuleRequest),
    DeliveryMargins(DeliveryMarginsSummary),
    RevenueByPlatform(RevenueByPlatformReport),
}

/// One imported bank-statement row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankTransactionRow {
    pub transaction_date: String,
    pub description: String,
    pub amount: f64,
    pub bank: Option<String>,
    pub account: Option<String>,
    pub reconciled: bool,
}

/// `IMPORT BANK STATEMENT <csv>` result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankTransactionImport {
    pub kind: BankingRequestKind,
    pub count: usize,
    pub rows: Vec<BankTransactionRow>,
}

/// One imported delivery-platform order row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeliveryTransactionRow {
    pub platform: String,
    pub platform_order_id: String,
    pub order_date: String,
    pub customer_name: Option<String>,
    pub subtotal: f64,
    pub delivery_fee: f64,
    pub platform_commission: f64,
    pub net_value: f64,
    pub reconciled: bool,
    pub status: String,
}

/// `IMPORT PLATFORM ORDERS <csv>` result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeliveryTransactionImport {
    pub kind: BankingRequestKind,
    pub count: usize,
    pub rows: Vec<DeliveryTransactionRow>,
}

/// `RECONCILE <deliveries>, <bank>` result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconciliationSummary {
    pub matched_count: usize,
    pub unmatched_count: usize,
    pub total_amount_matched: f64,
    pub matched: Vec<Value>,
    pub unmatched: Vec<Value>,
}

/// `GET UNMATCHED DELIVERIES <ids>` query filter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnmatchedDeliveriesQuery {
    pub kind: BankingRequestKind,
    pub filter: UnmatchedFilter,
}

/// Filter selecting the deliveries that still need reconciliation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmatchedFilter {
    pub reconciled: bool,
    pub exclude_ids: Vec<String>,
}

/// `ADD RECONCILE RULE <name>, <field>, <value>` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconciliationRuleRequest {
    pub kind: BankingRequestKind,
    pub name: String,
    pub match_field: String,
    pub match_operator: String,
    pub match_value: String,
    pub is_active: bool,
}

/// `DELIVERY MARGINS <deliveries>` aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeliveryMarginsSummary {
    pub count: usize,
    pub total_subtotal: f64,
    pub total_commission: f64,
    pub total_delivery_fee: f64,
    pub total_net: f64,
    pub margin_percent: f64,
}

/// One platform's slice of the revenue report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlatformRevenue {
    pub platform: String,
    pub net_revenue: f64,
    pub subtotal: f64,
    pub count: i64,
}

/// `REPORT REVENUE BY PLATFORM <deliveries>` report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevenueByPlatformReport {
    pub kind: BankingRequestKind,
    pub platforms: Vec<PlatformRevenue>,
}

impl BankingRequest {
    /// `IMPORT BANK STATEMENT <csv>` — normalise a bank export into rows.
    pub fn import_bank_statement(csv: &str) -> Self {
        Self::BankTransactions(import::import_bank_statement(csv))
    }

    /// `IMPORT PLATFORM ORDERS <csv>` — normalise a delivery export into rows.
    pub fn import_platform_orders(csv: &str) -> Self {
        Self::DeliveryTransactions(import::import_platform_orders(csv))
    }

    /// `RECONCILE <deliveries>, <bank>` — match delivery net values to bank
    /// transactions by amount or embedded order id.
    pub fn reconcile(deliveries_json: &str, bank_json: &str) -> Self {
        Self::Reconciliation(reconcile(deliveries_json, bank_json))
    }

    /// `GET UNMATCHED DELIVERIES <reconciled ids>`.
    pub fn unmatched_deliveries(reconciled_ids_json: &str) -> Self {
        let exclude_ids: Vec<String> =
            serde_json::from_str(reconciled_ids_json).unwrap_or_default();
        Self::UnmatchedQuery(UnmatchedDeliveriesQuery {
            kind: BankingRequestKind::UnmatchedQuery,
            filter: UnmatchedFilter {
                reconciled: false,
                exclude_ids,
            },
        })
    }

    /// `ADD RECONCILE RULE <name>, <field>, <value>`.
    pub fn add_reconcile_rule(name: &str, match_field: &str, match_value: &str) -> Self {
        Self::ReconciliationRule(ReconciliationRuleRequest {
            kind: BankingRequestKind::ReconciliationRule,
            name: name.to_string(),
            match_field: match_field.to_string(),
            match_operator: "contains".to_string(),
            match_value: match_value.to_string(),
            is_active: true,
        })
    }

    /// `DELIVERY MARGINS <deliveries>`.
    pub fn delivery_margins(deliveries_json: &str) -> Self {
        Self::DeliveryMargins(delivery_margins(deliveries_json))
    }

    /// `REPORT REVENUE BY PLATFORM <deliveries>`.
    pub fn revenue_by_platform(deliveries_json: &str) -> Self {
        Self::RevenueByPlatform(revenue_by_platform(deliveries_json))
    }

    /// JSON string handed back to the BASIC runtime.
    pub fn to_json_string(&self) -> String {
        serde_json::to_value(self)
            .map(|value| value.to_string())
            .unwrap_or_default()
    }
}

/// Parses the delivered-orders JSON, logging when it cannot be decoded.
fn parse_deliveries(deliveries_json: &str) -> Vec<Value> {
    match serde_json::from_str::<Vec<Value>>(deliveries_json) {
        Ok(deliveries) => deliveries,
        Err(e) => {
            log::warn!("banking intent: deliveries JSON rejected: {e}");
            Vec::new()
        }
    }
}

fn reconcile(deliveries_json: &str, bank_json: &str) -> ReconciliationSummary {
    let deliveries = parse_deliveries(deliveries_json);
    let bank = parse_deliveries(bank_json);

    let mut matched: Vec<Value> = Vec::new();
    let mut used: Vec<bool> = vec![false; bank.len()];
    for d in &deliveries {
        let d_amount = d.get("net_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let d_date = d.get("order_date").and_then(|v| v.as_str()).unwrap_or("");
        let d_order_id = d
            .get("platform_order_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        for (i, b) in bank.iter().enumerate() {
            if used[i] {
                continue;
            }
            let b_amount = b.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b_desc = b.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let amount_diff = (b_amount - d_amount).abs();
            let id_match = !d_order_id.is_empty() && b_desc.contains(d_order_id);
            if amount_diff < 0.01 || id_match {
                used[i] = true;
                matched.push(json!({
                    "delivery": d,
                    "bank": b,
                    "match_type": if id_match { "order_id" } else { "amount" },
                    "delivery_date": d_date,
                }));
                break;
            }
        }
    }

    let unmatched: Vec<Value> = bank
        .iter()
        .enumerate()
        .filter(|(i, _)| !used[*i])
        .map(|(_, v)| v.clone())
        .collect();

    let total: f64 = matched
        .iter()
        .filter_map(|m| m.get("delivery")?.get("net_value")?.as_f64())
        .sum();

    ReconciliationSummary {
        matched_count: matched.len(),
        unmatched_count: unmatched.len(),
        total_amount_matched: total,
        matched,
        unmatched,
    }
}

fn delivery_margins(deliveries_json: &str) -> DeliveryMarginsSummary {
    let deliveries = parse_deliveries(deliveries_json);

    let mut total_subtotal = 0.0;
    let mut total_commission = 0.0;
    let mut total_delivery_fee = 0.0;
    let mut total_net = 0.0;
    let mut count = 0;
    for d in &deliveries {
        total_subtotal += d.get("subtotal").and_then(|v| v.as_f64()).unwrap_or(0.0);
        total_commission += d
            .get("platform_commission")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        total_delivery_fee += d
            .get("delivery_fee")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        total_net += d.get("net_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        count += 1;
    }

    let margin_pct = if total_subtotal > 0.0 {
        (total_net / total_subtotal) * 100.0
    } else {
        0.0
    };

    DeliveryMarginsSummary {
        count,
        total_subtotal,
        total_commission,
        total_delivery_fee,
        total_net,
        margin_percent: margin_pct,
    }
}

fn revenue_by_platform(deliveries_json: &str) -> RevenueByPlatformReport {
    let deliveries = parse_deliveries(deliveries_json);
    let mut groups: BTreeMap<String, (f64, f64, i64)> = BTreeMap::new();
    for d in &deliveries {
        let platform = d
            .get("platform")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let net = d.get("net_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let subtotal = d.get("subtotal").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let entry = groups.entry(platform).or_insert((0.0, 0.0, 0));
        entry.0 += net;
        entry.1 += subtotal;
        entry.2 += 1;
    }
    let platforms: Vec<PlatformRevenue> = groups
        .into_iter()
        .map(
            |(platform, (net_revenue, subtotal, count))| PlatformRevenue {
                platform,
                net_revenue,
                subtotal,
                count,
            },
        )
        .collect();
    RevenueByPlatformReport {
        kind: BankingRequestKind::RevenueByPlatform,
        platforms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(request: &BankingRequest) -> Value {
        serde_json::to_value(request).expect("serialize")
    }

    #[test]
    fn reconcile_matches_by_amount_and_reports_unmatched_bank_rows() {
        let deliveries =
            r#"[{"platform_order_id":"A1","order_date":"2026-08-01","net_value":100.0}]"#;
        let bank =
            r#"[{"description":"PIX A1","amount":100.0},{"description":"fee","amount":7.5}]"#;
        let json = value(&BankingRequest::reconcile(deliveries, bank));
        assert_eq!(json["matched_count"], 1);
        assert_eq!(json["unmatched_count"], 1);
        assert_eq!(json["total_amount_matched"], 100.0);
        assert_eq!(json["matched"][0]["match_type"], "order_id");
    }

    #[test]
    fn reconcile_matches_by_amount_when_no_reference() {
        let deliveries =
            r#"[{"platform_order_id":"","order_date":"2026-08-01","net_value":42.42}]"#;
        let bank = r#"[{"description":"no ref","amount":42.42}]"#;
        let json = value(&BankingRequest::reconcile(deliveries, bank));
        assert_eq!(json["matched"][0]["match_type"], "amount");
    }

    #[test]
    fn reconcile_tolerates_malformed_payloads() {
        let json = value(&BankingRequest::reconcile("not json", "{}"));
        assert_eq!(json["matched_count"], 0);
        assert_eq!(json["unmatched_count"], 0);
    }

    #[test]
    fn margins_aggregate_totals_and_percentage() {
        let deliveries = r#"[
            {"subtotal":100.0,"platform_commission":20.0,"delivery_fee":5.0,"net_value":80.0},
            {"subtotal":300.0,"platform_commission":60.0,"delivery_fee":0.0,"net_value":240.0}
        ]"#;
        let json = value(&BankingRequest::delivery_margins(deliveries));
        assert_eq!(json["count"], 2);
        assert_eq!(json["total_subtotal"], 400.0);
        assert_eq!(json["total_commission"], 80.0);
        assert_eq!(json["total_net"], 320.0);
        assert_eq!(json["margin_percent"], 80.0);
    }

    #[test]
    fn margins_without_subtotal_report_zero_percentage() {
        let json = value(&BankingRequest::delivery_margins("[]"));
        assert_eq!(json["count"], 0);
        assert_eq!(json["margin_percent"], 0.0);
    }

    #[test]
    fn revenue_groups_by_platform_in_deterministic_order() {
        let deliveries = r#"[
            {"platform":"ifood","net_value":10.0,"subtotal":12.0},
            {"platform":"rappi","net_value":5.0,"subtotal":6.0},
            {"platform":"ifood","net_value":1.0,"subtotal":2.0}
        ]"#;
        let json = value(&BankingRequest::revenue_by_platform(deliveries));
        assert_eq!(json["kind"], "revenue_by_platform");
        assert_eq!(json["platforms"][0]["platform"], "ifood");
        assert_eq!(json["platforms"][0]["net_revenue"], 11.0);
        assert_eq!(json["platforms"][0]["count"], 2);
        assert_eq!(json["platforms"][1]["platform"], "rappi");
    }

    #[test]
    fn revenue_defaults_unknown_platform() {
        let json = value(&BankingRequest::revenue_by_platform(
            r#"[{"net_value":1.0}]"#,
        ));
        assert_eq!(json["platforms"][0]["platform"], "unknown");
    }

    #[test]
    fn unmatched_deliveries_query_excludes_reconciled_ids() {
        let json = value(&BankingRequest::unmatched_deliveries(r#"["a","b"]"#));
        assert_eq!(json["kind"], "unmatched_query");
        assert_eq!(json["filter"]["reconciled"], false);
        assert_eq!(json["filter"]["exclude_ids"][0], "a");
    }

    #[test]
    fn reconcile_rule_is_active_and_contains() {
        let json = value(&BankingRequest::add_reconcile_rule(
            "Ifood PIX",
            "description",
            "IFOOD",
        ));
        assert_eq!(json["kind"], "reconciliation_rule");
        assert_eq!(json["match_operator"], "contains");
        assert_eq!(json["is_active"], true);
    }
}
