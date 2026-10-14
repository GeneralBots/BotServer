//! Value formatting, argument splitting and criteria matching.

use crate::types::Worksheet;

use super::refs::parse_cell_ref;

pub fn format_number(num: f64) -> String {
    if num.fract() == 0.0 {
        // `{:.0}` (not `as i64`) — an `as` cast saturates for magnitudes beyond
        // i64 (e.g. =10^20), silently showing a wrong value.
        format!("{:.0}", num)
    } else {
        format!("{:.6}", num)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

pub fn resolve_cell_value(value: &str, worksheet: &Worksheet) -> String {
    if let Some((row, col)) = parse_cell_ref(value) {
        let key = format!("{},{}", row, col);
        worksheet
            .data
            .get(&key)
            .and_then(|c| c.value.clone())
            .unwrap_or_default()
    } else {
        value.to_string()
    }
}

pub fn split_args(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

pub fn evaluate_condition(condition: &str, worksheet: &Worksheet) -> bool {
    let condition = condition.trim();
    if condition.eq_ignore_ascii_case("TRUE") {
        return true;
    }
    if condition.eq_ignore_ascii_case("FALSE") {
        return false;
    }

    let operators = [">=", "<=", "<>", "!=", "=", ">", "<"];
    for op in &operators {
        if let Some(pos) = condition.find(op) {
            let left = resolve_cell_value(condition[..pos].trim(), worksheet);
            let right = resolve_cell_value(
                condition[pos + op.len()..].trim().trim_matches('"'),
                worksheet,
            );

            let left_num = left.parse::<f64>().ok();
            let right_num = right.parse::<f64>().ok();

            return match (*op, left_num, right_num) {
                (">=", Some(l), Some(r)) => l >= r,
                ("<=", Some(l), Some(r)) => l <= r,
                ("<>" | "!=", Some(l), Some(r)) => (l - r).abs() > f64::EPSILON,
                ("<>" | "!=", _, _) => left != right,
                ("=", Some(l), Some(r)) => (l - r).abs() < f64::EPSILON,
                ("=", _, _) => left.eq_ignore_ascii_case(&right),
                (">", Some(l), Some(r)) => l > r,
                ("<", Some(l), Some(r)) => l < r,
                _ => false,
            };
        }
    }
    false
}

pub fn matches_criteria(value: &str, criteria: &str) -> bool {
    if let Some(rest) = criteria.strip_prefix(">=") {
        if let (Ok(v), Ok(c)) = (value.parse::<f64>(), rest.parse::<f64>()) {
            return v >= c;
        }
    } else if let Some(rest) = criteria.strip_prefix("<=") {
        if let (Ok(v), Ok(c)) = (value.parse::<f64>(), rest.parse::<f64>()) {
            return v <= c;
        }
    } else if criteria.starts_with("<>") || criteria.starts_with("!=") {
        let c = &criteria[2..];
        return !value.eq_ignore_ascii_case(c);
    } else if let Some(rest) = criteria.strip_prefix('>') {
        if let (Ok(v), Ok(c)) = (value.parse::<f64>(), rest.parse::<f64>()) {
            return v > c;
        }
    } else if let Some(rest) = criteria.strip_prefix('<') {
        if let (Ok(v), Ok(c)) = (value.parse::<f64>(), rest.parse::<f64>()) {
            return v < c;
        }
    } else if let Some(rest) = criteria.strip_prefix('=') {
        return value.eq_ignore_ascii_case(rest);
    } else if criteria.contains('*') || criteria.contains('?') {
        let pattern = criteria.replace('*', ".*").replace('?', ".");
        if let Some(re) = crate::re_cache::dynamic(&format!("^{}$", pattern)) {
            return re.is_match(value);
        }
    }
    value.eq_ignore_ascii_case(criteria)
}

pub fn count_matching(values: &[String], criteria: &str) -> usize {
    values
        .iter()
        .filter(|v| matches_criteria(v, criteria))
        .count()
}

