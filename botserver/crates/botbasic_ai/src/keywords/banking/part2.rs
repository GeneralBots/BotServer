use super::*;

pub(super) fn register_get_unmatched_deliveries(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "UNMATCHED", "DELIVERIES", "$expr$"],
            false,
            move |context, inputs| {
                let reconciled_ids_json = context.eval_expression_tree(&inputs[0])?.to_string();
                trace!("GET UNMATCHED DELIVERIES");
                let reconciled: Vec<String> =
                    serde_json::from_str(&reconciled_ids_json).unwrap_or_default();
                let result = json!({
                    "kind": "unmatched_query",
                    "filter": { "reconciled": false, "exclude_ids": reconciled },
                });
                Ok(serde_json_to_dynamic(&result))
            },
        )
    {
        log::error!("valid GET UNMATCHED DELIVERIES syntax failed: {e}");
    }
}
pub(super) fn register_add_reconcile_rule(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["ADD", "RECONCILE", "RULE", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let name = context.eval_expression_tree(&inputs[0])?.to_string();
                let match_field = context.eval_expression_tree(&inputs[1])?.to_string();
                let match_value = context.eval_expression_tree(&inputs[2])?.to_string();
                trace!("ADD RECONCILE RULE: {name} {match_field}={match_value}");
                let result = json!({
                    "kind": "reconciliation_rule",
                    "name": name,
                    "match_field": match_field,
                    "match_operator": "contains",
                    "match_value": match_value,
                    "is_active": true,
                });
                Ok(serde_json_to_dynamic(&result))
            },
        )
    {
        log::error!("valid ADD RECONCILE RULE syntax failed: {e}");
    }
}
pub(super) fn register_delivery_margins(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(["DELIVERY", "MARGINS", "$expr$"], false, move |context, inputs| {
            let deliveries_json = context.eval_expression_tree(&inputs[0])?.to_string();
            trace!("DELIVERY MARGINS");
            let deliveries: Vec<Value> =
                serde_json::from_str(&deliveries_json).unwrap_or_default();

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
                total_delivery_fee += d.get("delivery_fee").and_then(|v| v.as_f64()).unwrap_or(0.0);
                total_net += d.get("net_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                count += 1;
            }

            let margin_pct = if total_subtotal > 0.0 {
                (total_net / total_subtotal) * 100.0
            } else {
                0.0
            };

            let result = json!({
                "count": count,
                "total_subtotal": total_subtotal,
                "total_commission": total_commission,
                "total_delivery_fee": total_delivery_fee,
                "total_net": total_net,
                "margin_percent": margin_pct,
            });
            Ok(serde_json_to_dynamic(&result))
        })
    {
        log::error!("valid DELIVERY MARGINS syntax failed: {e}");
    }
}
pub(super) fn register_revenue_by_platform(
    state: Arc<dyn BasicRuntime>,
    _user: UserSession,
    engine: &mut Engine,
) {
    let _state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["REPORT", "REVENUE", "BY", "PLATFORM", "$expr$"],
            false,
            move |context, inputs| {
                let deliveries_json = context.eval_expression_tree(&inputs[0])?.to_string();
                trace!("REPORT REVENUE BY PLATFORM");
                let deliveries: Vec<Value> =
                    serde_json::from_str(&deliveries_json).unwrap_or_default();
                let mut groups: std::collections::HashMap<String, (f64, f64, i64)> =
                    std::collections::HashMap::new();
                for d in &deliveries {
                    let p = d
                        .get("platform")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let net = d.get("net_value").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let subtotal = d.get("subtotal").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let entry = groups.entry(p).or_insert((0.0, 0.0, 0));
                    entry.0 += net;
                    entry.1 += subtotal;
                    entry.2 += 1;
                }
                let breakdown: Vec<Value> = groups
                    .into_iter()
                    .map(|(platform, (net, subtotal, count))| {
                        json!({
                            "platform": platform,
                            "net_revenue": net,
                            "subtotal": subtotal,
                            "count": count,
                        })
                    })
                    .collect();
                let result = json!({
                    "kind": "revenue_by_platform",
                    "platforms": breakdown,
                });
                Ok(serde_json_to_dynamic(&result))
            },
        )
    {
        log::error!("valid REPORT REVENUE BY PLATFORM syntax failed: {e}");
    }
}
pub(super) fn serde_json_to_dynamic(v: &Value) -> Dynamic {
    let s = v.to_string();
    Dynamic::from(s)
}
