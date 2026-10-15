use crate::types::Worksheet;

use super::helpers::{format_number, split_args};

fn group_rows(values: &[String], group_by_idx: usize) -> std::collections::BTreeMap<String, Vec<usize>> {
    let mut groups: std::collections::BTreeMap<String, Vec<usize>> = std::collections::BTreeMap::new();
    for (i, chunk) in values.chunks(2).enumerate() {
        if let Some(key) = chunk.get(group_by_idx) {
            groups.entry(key.clone()).or_default().push(i);
        }
    }
    groups
}

pub fn evaluate_groupby(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("GROUPBY(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[8..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 3 {
        return Some("#ERROR!".to_string());
    }
    let row_data = super::get_range_string_values(parts[0].trim(), worksheet);
    let group_idx: usize = parts[1].trim().parse().unwrap_or(0);
    let agg = parts[2].trim().trim_matches('"').to_uppercase();
    let groups = group_rows(&row_data, group_idx);
    let mut out: Vec<String> = Vec::new();
    for (key, indices) in &groups {
        let vals: Vec<f64> = indices.iter().filter_map(|&i| {
            let nums: Vec<f64> = row_data.chunks(2).nth(i).map(|c| c.iter().filter_map(|s| s.parse().ok()).collect()).unwrap_or_default();
            nums.first().copied()
        }).collect();
        let agg_val = match agg.as_str() {
            "SUM" => vals.iter().sum::<f64>(),
            "AVERAGE" | "AVG" | "MEAN" => if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 },
            "COUNT" => vals.len() as f64,
            "MAX" => vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            "MIN" => vals.iter().cloned().fold(f64::INFINITY, f64::min),
            "MEDIAN" => {
                let mut s = vals.clone();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                if s.is_empty() { 0.0 } else { s[s.len() / 2] }
            }
            _ => 0.0,
        };
        out.push(format!("{}:{}", key, format_number(agg_val)));
    }
    Some(out.join("|"))
}

pub fn evaluate_pivotby(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("PIVOTBY(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[8..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 4 {
        return Some("#ERROR!".to_string());
    }
    let row_data = super::get_range_string_values(parts[0].trim(), worksheet);
    let col_data = super::get_range_string_values(parts[1].trim(), worksheet);
    let value_data = super::get_range_string_values(parts[2].trim(), worksheet);
    let agg = parts[3].trim().trim_matches('"').to_uppercase();
    let mut cells: std::collections::BTreeMap<(String, String), Vec<f64>> = std::collections::BTreeMap::new();
    for ((row_key, col_key), value) in row_data.iter().zip(col_data.iter()).zip(value_data.iter()) {
        if let Ok(v) = value.parse::<f64>() {
            cells
                .entry((row_key.clone(), col_key.clone()))
                .or_default()
                .push(v);
        }
    }
    let mut out: Vec<String> = Vec::new();
    for ((row_key, col_key), vals) in &cells {
        let agg_val = match agg.as_str() {
            "SUM" => vals.iter().sum::<f64>(),
            "AVERAGE" | "AVG" | "MEAN" => if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 },
            "COUNT" => vals.len() as f64,
            "MAX" => vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            "MIN" => vals.iter().cloned().fold(f64::INFINITY, f64::min),
            "MEDIAN" => {
                let mut s = vals.clone();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                if s.is_empty() { 0.0 } else { s[s.len() / 2] }
            }
            _ => 0.0,
        };
        out.push(format!("{}|{}|{}", row_key, col_key, format_number(agg_val)));
    }
    Some(out.join(","))
}

pub fn evaluate_percentof(expr: &str, _worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("PERCENTOF(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[11..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let value = parts[0].trim().parse::<f64>().unwrap_or(0.0);
    let total = parts[1].trim().parse::<f64>().unwrap_or(1.0);
    if total == 0.0 {
        return Some("#DIV/0!".to_string());
    }
    Some(format_number((value / total) * 100.0) + "%")
}

pub fn evaluate_subtotal(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("SUBTOTAL(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[9..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let fnum: i64 = parts[0].trim().parse().unwrap_or(0);
    let values = super::get_range_values(parts[1].trim(), worksheet);
    let filtered: Vec<f64> = if fnum >= 100 {
        values.into_iter().filter(|&v| !v.is_nan()).collect()
    } else {
        values.into_iter().filter(|&v| !v.is_nan() && v != 0.0).collect()
    };
    let result = match fnum % 100 {
        1 | 101 => filtered.iter().sum::<f64>(),
        2 | 102 => filtered.len() as f64,
        3 | 103 => filtered.len() as f64,
        4 | 104 => if filtered.is_empty() { 0.0 } else { filtered.iter().cloned().fold(f64::NEG_INFINITY, f64::max) },
        5 | 105 => if filtered.is_empty() { 0.0 } else { filtered.iter().cloned().fold(f64::INFINITY, f64::min) },
        9 | 109 => if filtered.is_empty() { 0.0 } else { filtered.iter().sum::<f64>() / filtered.len() as f64 },
        _ => 0.0,
    };
    Some(format_number(result))
}

pub fn evaluate_aggregate(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("AGGREGATE(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[10..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let fnum: i64 = parts[0].trim().parse().unwrap_or(0);
    let values = super::get_range_values(parts[1].trim(), worksheet);
    let result = match fnum {
        1 => values.iter().sum::<f64>(),
        2 => values.len() as f64,
        3 => values.len() as f64,
        4 => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        5 => values.iter().cloned().fold(f64::INFINITY, f64::min),
        9 => if values.is_empty() { 0.0 } else { values.iter().sum::<f64>() / values.len() as f64 },
        _ => 0.0,
    };
    Some(format_number(result))
}

pub fn evaluate_percentile(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("PERCENTILE(") && !expr.starts_with("PERCENTILE.INC(") {
        return None;
    }
    let start = if expr.starts_with("PERCENTILE.INC(") { 16 } else { 11 };
    if !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[start..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let mut values = super::get_range_values(parts[0].trim(), worksheet);
    let p: f64 = parts[1].trim().parse().unwrap_or(0.5);
    if values.is_empty() {
        return Some("#NUM!".to_string());
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = (p * (values.len() - 1) as f64).round() as usize;
    Some(format_number(values[idx.min(values.len() - 1)]))
}

pub fn evaluate_quartile(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("QUARTILE(") && !expr.starts_with("QUARTILE.INC(") {
        return None;
    }
    let start = if expr.starts_with("QUARTILE.INC(") { 14 } else { 9 };
    if !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[start..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let mut values = super::get_range_values(parts[0].trim(), worksheet);
    let q: f64 = parts[1].trim().parse().unwrap_or(0.0);
    if values.is_empty() {
        return Some("#NUM!".to_string());
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = (q * (values.len() - 1) as f64 / 4.0).round() as usize;
    Some(format_number(values[idx.min(values.len() - 1)]))
}

pub fn evaluate_rank(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("RANK(") && !expr.starts_with("RANK.EQ(") {
        return None;
    }
    let start = if expr.starts_with("RANK.EQ(") { 9 } else { 5 };
    if !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[start..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let value: f64 = parts[0].trim().parse().unwrap_or(0.0);
    let values = super::get_range_values(parts[1].trim(), worksheet);
    let order = if parts.len() > 2 { parts[2].trim().parse::<i64>().unwrap_or(0) } else { 0 };
    let mut sorted = values.clone();
    if order == 0 { sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); }
    else { sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)); }
    let rank = sorted.iter().position(|&v| (v - value).abs() < f64::EPSILON).map(|i| i + 1).unwrap_or(0);
    Some(rank.to_string())
}
