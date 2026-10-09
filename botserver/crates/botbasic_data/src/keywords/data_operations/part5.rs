use super::*;

pub(super) fn execute_filter(data: &Dynamic, condition: &str) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    let (field, operator, value) = parse_condition(condition)?;
    let array = to_array(data);
    let mut results: Array = Array::new();

    for item in array {
        let item_map = dynamic_to_map(&item);

        if let Some(field_value) = item_map.get(&field) {
            let matches = match operator.as_str() {
                "=" | "==" => field_value.to_string() == value,
                "!=" | "<>" => field_value.to_string() != value,
                ">" => {
                    field_value.to_string().parse::<f64>().unwrap_or(0.0)
                        > value.parse::<f64>().unwrap_or(0.0)
                }
                "<" => {
                    field_value.to_string().parse::<f64>().unwrap_or(0.0)
                        < value.parse::<f64>().unwrap_or(0.0)
                }
                ">=" => {
                    field_value.to_string().parse::<f64>().unwrap_or(0.0)
                        >= value.parse::<f64>().unwrap_or(0.0)
                }
                "<=" => {
                    field_value.to_string().parse::<f64>().unwrap_or(0.0)
                        <= value.parse::<f64>().unwrap_or(0.0)
                }
                "like" | "LIKE" => field_value
                    .to_string()
                    .to_lowercase()
                    .contains(&value.to_lowercase()),
                _ => false,
            };

            if matches {
                results.push(item.clone());
            }
        }
    }

    Ok(Dynamic::from(results))
}
pub(super) fn execute_aggregate(
    operation: &str,
    data: &Dynamic,
    field: &str,
) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    let array = to_array(data);
    let mut values: Vec<f64> = Vec::new();

    for item in array {
        let item_map = dynamic_to_map(&item);
        if let Some(field_value) = item_map.get(field) {
            if let Ok(num) = field_value.to_string().parse::<f64>() {
                values.push(num);
            }
        }
    }

    let result = match operation.to_uppercase().as_str() {
        "SUM" => values.iter().sum::<f64>(),
        "AVG" | "AVERAGE" => {
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        }
        "COUNT" => values.len() as f64,
        "MIN" => values.iter().copied().fold(f64::INFINITY, f64::min),
        "MAX" => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        _ => {
            return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                format!("Unknown aggregate operation: {}", operation).into(),
                rhai::Position::NONE,
            )));
        }
    };

    Ok(Dynamic::from(result))
}
pub(super) fn execute_join(
    left: &Dynamic,
    right: &Dynamic,
    key: &str,
) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    let left_array = to_array(left);
    let right_array = to_array(right);
    let mut results: Array = Array::new();

    let mut right_index: HashMap<String, Vec<Map>> = HashMap::new();
    for item in &right_array {
        let item_map = dynamic_to_map(item);
        if let Some(key_value) = item_map.get(key) {
            let key_str = key_value.to_string();
            right_index
                .entry(key_str)
                .or_default()
                .push(dynamic_to_rhai_map(item));
        }
    }

    for left_item in &left_array {
        let left_map = dynamic_to_map(left_item);
        if let Some(key_value) = left_map.get(key) {
            let key_str = key_value.to_string();
            if let Some(right_matches) = right_index.get(&key_str) {
                for right_map in right_matches {
                    let mut joined_map: Map = dynamic_to_rhai_map(left_item);
                    for (k, v) in right_map {
                        if !joined_map.contains_key(k.as_str()) {
                            joined_map.insert(k.clone(), v.clone());
                        }
                    }
                    results.push(Dynamic::from(joined_map));
                }
            }
        }
    }

    Ok(Dynamic::from(results))
}
pub(super) fn execute_pivot(
    data: &Dynamic,
    row_field: &str,
    value_field: &str,
) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    let array = to_array(data);
    let mut pivot: HashMap<String, f64> = HashMap::new();

    for item in array {
        let item_map = dynamic_to_map(&item);

        let row_key = item_map
            .get(row_field)
            .map(|v| v.to_string())
            .unwrap_or_default();

        let value = item_map
            .get(value_field)
            .and_then(|v| v.to_string().parse::<f64>().ok())
            .unwrap_or(0.0);

        *pivot.entry(row_key).or_insert(0.0) += value;
    }

    let mut results: Array = Array::new();
    for (key, sum) in pivot {
        let mut row: Map = Map::new();
        row.insert(row_field.into(), Dynamic::from(key));
        row.insert(value_field.into(), Dynamic::from(sum));
        results.push(Dynamic::from(row));
    }

    Ok(Dynamic::from(results))
}
pub(super) fn execute_group_by(data: &Dynamic, field: &str) -> Result<Dynamic, Box<rhai::EvalAltResult>> {
    let array = to_array(data);
    let mut groups: HashMap<String, Array> = HashMap::new();

    for item in array {
        let item_map = dynamic_to_map(&item);

        let group_key = item_map
            .get(field)
            .map(|v| v.to_string())
            .unwrap_or_default();

        groups.entry(group_key).or_default().push(item);
    }

    let mut result_map: Map = Map::new();
    for (key, items) in groups {
        result_map.insert(key.into(), Dynamic::from(items));
    }

    Ok(Dynamic::from(result_map))
}
pub(super) fn dynamic_to_map(value: &Dynamic) -> HashMap<String, Dynamic> {
    let mut result = HashMap::new();

    if value.is_map() {
        let map = value.clone().cast::<Map>();
        for (k, v) in map.iter() {
            result.insert(k.to_string(), v.clone());
        }
    }

    result
}
pub(super) fn dynamic_to_rhai_map(value: &Dynamic) -> Map {
    if value.is_map() {
        value.clone().cast::<Map>()
    } else {
        Map::new()
    }
}
pub(super) fn parse_filter_clause(filter: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let (field, operator, value) = parse_condition_internal(filter)?;

    let sql_operator = match operator.as_str() {
        "!=" | "<>" => "!=",
        ">" => ">",
        "<" => "<",
        ">=" => ">=",
        "<=" => "<=",
        "like" | "LIKE" => "LIKE",
        _ => "=",
    };

    Ok(format!(
        "{} {} '{}'",
        sanitize_identifier(&field),
        sql_operator,
        sanitize_sql_value(&value)
    ))
}
pub(super) fn parse_condition(condition: &str) -> Result<(String, String, String), Box<rhai::EvalAltResult>> {
    parse_condition_internal(condition).map_err(|e| {
        Box::new(rhai::EvalAltResult::ErrorRuntime(
            e.to_string().into(),
            rhai::Position::NONE,
        ))
    })
}
pub(super) fn parse_condition_internal(
    condition: &str,
) -> Result<(String, String, String), Box<dyn Error + Send + Sync>> {
    let operators = [">=", "<=", "!=", "<>", "==", "=", ">", "<", "like", "LIKE"];

    for op in operators {
        if let Some(pos) = condition.find(op) {
            let field = condition[..pos].trim().to_string();
            let value = condition[pos + op.len()..].trim().to_string();
            return Ok((field, op.to_string(), value));
        }
    }

    Err("Invalid condition format".into())
}
