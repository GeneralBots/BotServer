use crate::types::Worksheet;

use super::helpers::{format_number, resolve_cell_value, split_args};

pub fn evaluate_let(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("LET(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[4..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 3 || parts.len().is_multiple_of(2) {
        return Some("#ERROR!".to_string());
    }
    let mut bindings: Vec<(String, String)> = Vec::new();
    for chunk in parts.as_chunks::<2>().0 {
        let name = chunk[0].trim().to_string();
        let val_expr = chunk[1].trim();
        let val = if val_expr.starts_with('=') {
            super::evaluate_formula(val_expr, worksheet).value
        } else {
            resolve_cell_value(val_expr, worksheet)
        };
        bindings.push((name, val));
    }
    let calc = parts[parts.len() - 1].trim();
    let substituted = apply_bindings(calc, &bindings);
    Some(eval_substituted(&substituted, worksheet))
}

pub fn evaluate_lambda(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("LAMBDA(") {
        return None;
    }
    // Locate the `)` that closes the LAMBDA(...) body; anything after it is
    // a direct invocation `(arg1, arg2, ...)`.
    let body_end = find_matching_paren(expr, 6);
    if body_end == 0 {
        return None;
    }
    let body = &expr[7..body_end];
    let tail = expr[body_end + 1..].trim();
    let parts = split_args(body);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    // Params are the leading identifier parts: split_args splits every
    // top-level comma, so `LAMBDA(a,b,a+b)` yields ["a", "b", "a+b"] —
    // the first two parts are the params, the tail is the body. A bare
    // definition like `LAMBDA(a,b,a+b)` without invocation must still parse.
    let param_names: Vec<String> = parts
        .iter()
        .take_while(|p| {
            let t = p.trim();
            !t.is_empty()
                && t.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        })
        .map(|s| s.trim().to_string())
        .collect();
    if param_names.is_empty() || param_names.len() >= parts.len() {
        return Some("#ERROR!".to_string());
    }
    // The body is everything AFTER the parameter list.
    let calc = parts[param_names.len()..].join(",");
    // A bare definition is a function value in the string-cell model.
    if tail.is_empty() || !tail.starts_with('(') || !tail.ends_with(')') {
        return Some(format!("λ({})", parts[0].trim()));
    }
    let args = split_args(&tail[1..tail.len() - 1]);
    if args.len() != param_names.len() {
        return Some("#ERROR!".to_string());
    }
    let bindings: Vec<(String, String)> = param_names
        .iter()
        .zip(args.iter())
        .map(|(name, arg)| (name.clone(), arg.trim().to_string()))
        .collect();
    let substituted = apply_bindings(calc.trim(), &bindings);
    Some(eval_substituted(&substituted, worksheet))
}

fn find_matching_paren(s: &str, open: usize) -> usize {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
        i += 1;
    }
    0
}

pub fn evaluate_map(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("MAP(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[4..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let Some(lambda) = parts.last() else {
        return Some("#ERROR!".to_string());
    };
    let lambda = lambda.trim();
    let mut values: Vec<String> = Vec::new();
    for p in &parts[..parts.len() - 1] {
        values.extend(super::get_range_string_values(p.trim(), worksheet));
    }
    let mut out: Vec<String> = Vec::new();
    for v in &values {
        let mut bindings: Vec<(String, String)> = Vec::new();
        if lambda.starts_with("LAMBDA(") {
            let l_inner = &lambda[7..lambda.len() - 1];
            let l_parts = split_args(l_inner);
            if !l_parts.is_empty() {
                let param_names = l_parts[0].trim();
                let calc = l_parts[1..].join(",");
                for name in param_names.split(',') {
                    bindings.push((name.trim().to_string(), v.clone()));
                }
                let sub = apply_bindings(&calc, &bindings);
                out.push(eval_substituted(&sub, worksheet));
            }
        } else {
            out.push(resolve_cell_value(lambda, worksheet));
        }
    }
    Some(out.join(","))
}

pub fn evaluate_reduce(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("REDUCE(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[7..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 3 {
        return Some("#ERROR!".to_string());
    }
    let initial = resolve_cell_value(parts[0].trim(), worksheet);
    let values = super::get_range_string_values(parts[1].trim(), worksheet);
    let lambda = parts[2].trim();
    let mut acc = initial;
    for v in values {
        let sub = reduce_bindings(lambda, &acc, &v).unwrap_or_else(|| {
            lambda
                .replace("[accumulator]", &acc)
                .replace("[value]", &v)
                .replace("[acc]", &acc)
                .replace("[val]", &v)
        });
        acc = eval_substituted(&sub, worksheet);
    }
    Some(acc)
}

pub fn evaluate_byrow(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("BYROW(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[6..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let range = parts[0].trim();
    let lambda = parts[1].trim();
    let rows = collect_rows(range, worksheet);
    let mut out: Vec<String> = Vec::new();
    for row in rows {
        let sub = lambda.replace("[row]", &row.join(","));
        let val = eval_substituted(&sub, worksheet);
        out.push(val);
    }
    Some(out.join(","))
}

pub fn evaluate_bycol(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("BYCOL(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[6..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return Some("#ERROR!".to_string());
    }
    let range = parts[0].trim();
    let lambda = parts[1].trim();
    let cols = collect_cols(range, worksheet);
    let mut out: Vec<String> = Vec::new();
    for col in cols {
        let sub = lambda.replace("[col]", &col.join(","));
        let val = eval_substituted(&sub, worksheet);
        out.push(val);
    }
    Some(out.join(","))
}

pub fn evaluate_makearray(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("MAKEARRAY(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[10..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 3 {
        return Some("#ERROR!".to_string());
    }
    let rows: i64 = parts[0].trim().parse().unwrap_or(0);
    let cols: i64 = parts[1].trim().parse().unwrap_or(0);
    let lambda = parts[2].trim();
    let mut out: Vec<String> = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let sub = lambda.replace("[row]", &r.to_string()).replace("[col]", &c.to_string());
            let v = eval_substituted(&sub, worksheet);
            out.push(v);
        }
    }
    Some(out.join(","))
}

fn apply_bindings(calc: &str, bindings: &[(String, String)]) -> String {
    let mut out = calc.to_string();
    for (name, val) in bindings {
        out = out.replace(name, val);
    }
    out
}

/// Evaluates a substituted lambda body: expressions go through the engine,
/// plain literals and cell references resolve directly.
fn eval_substituted(sub: &str, worksheet: &Worksheet) -> String {
    let s = sub.trim();
    if s.is_empty() {
        return String::new();
    }
    if s.starts_with('=') {
        return super::evaluate_formula(s, worksheet).value;
    }
    let is_expr = s
        .chars()
        .any(|c| matches!(c, '+' | '-' | '*' | '/' | '^' | '&' | '('));
    if is_expr || super::parse_cell_ref(s).is_none() {
        return super::evaluate_formula(&format!("={s}"), worksheet).value;
    }
    resolve_cell_value(s, worksheet)
}

/// Binds a REDUCE accumulator and current value to a `LAMBDA(acc,val,body)`
/// signature, falling back to the legacy `[acc]`/`[val]` placeholders.
fn reduce_bindings(lambda: &str, acc: &str, v: &str) -> Option<String> {
    if !lambda.starts_with("LAMBDA(") || !lambda.ends_with(')') {
        return None;
    }
    let inner = &lambda[7..lambda.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return None;
    }
    let mut bindings: Vec<(String, String)> = Vec::new();
    for (i, name) in parts[0].split(',').enumerate() {
        let val = if i == 0 { acc } else { v };
        bindings.push((name.trim().to_string(), val.to_string()));
    }
    let calc = parts[1..].join(",");
    Some(apply_bindings(calc.trim(), &bindings))
}

fn collect_rows(range: &str, worksheet: &Worksheet) -> Vec<Vec<String>> {
    if let Some((start, end)) = super::parse_range(range).map(|(s, e)| super::refs::clamp_range(s, e, worksheet)) {
        let mut out = Vec::new();
        for r in start.0..=end.0 {
            let mut row = Vec::new();
            for c in start.1..=end.1 {
                let key = format!("{},{}", r, c);
                if let Some(cell) = worksheet.data.get(&key) {
                    row.push(cell.value.clone().unwrap_or_default());
                } else {
                    row.push(String::new());
                }
            }
            out.push(row);
        }
        out
    } else {
        Vec::new()
    }
}

fn collect_cols(range: &str, worksheet: &Worksheet) -> Vec<Vec<String>> {
    if let Some((start, end)) = super::parse_range(range).map(|(s, e)| super::refs::clamp_range(s, e, worksheet)) {
        let mut out = Vec::new();
        for c in start.1..=end.1 {
            let mut col = Vec::new();
            for r in start.0..=end.0 {
                let key = format!("{},{}", r, c);
                if let Some(cell) = worksheet.data.get(&key) {
                    col.push(cell.value.clone().unwrap_or_default());
                } else {
                    col.push(String::new());
                }
            }
            out.push(col);
        }
        out
    } else {
        Vec::new()
    }
}

pub fn evaluate_reduce_arithmetic(expr: &str, worksheet: &Worksheet) -> Option<String> {
    if !expr.starts_with("REDUCE_OP(") || !expr.ends_with(')') {
        return None;
    }
    let inner = &expr[11..expr.len() - 1];
    let parts = split_args(inner);
    if parts.len() < 2 {
        return None;
    }
    let op = parts[0].trim().trim_matches('"');
    let values = super::get_range_values(parts[1].trim(), worksheet);
    let acc = if parts.len() > 2 {
        parts[2].trim().parse().unwrap_or(0.0)
    } else {
        0.0
    };
    let result = match op.to_uppercase().as_str() {
        "SUM" | "+" => values.iter().fold(acc, |a, b| a + b),
        "PRODUCT" | "*" => values.iter().fold(if acc == 0.0 { 1.0 } else { acc }, |a, b| a * b),
        "MAX" => values.iter().fold(acc, |a, b| a.max(*b)),
        "MIN" => values.iter().fold(acc, |a, b| a.min(*b)),
        "AND" => values.iter().fold(if acc == 0.0 { 1.0 } else { acc }, |a, b| if *b != 0.0 && a != 0.0 { 1.0 } else { 0.0 }),
        "OR" => values.iter().fold(acc, |a, b| if *b != 0.0 || a != 0.0 { 1.0 } else { 0.0 }),
        _ => return Some("#ERROR!".to_string()),
    };
    Some(format_number(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Worksheet;

    fn ev(formula: &str) -> Option<String> {
        evaluate_lambda(formula, &Worksheet::default())
    }

    #[test]
    fn bare_definition_returns_function_value() {
        assert_eq!(ev("LAMBDA(x,x*2)"), Some("λ(x)".to_string()));
    }

    #[test]
    fn direct_invocation_evaluates_body() {
        assert_eq!(ev("LAMBDA(x,x*2)(5)"), Some("10".to_string()));
    }

    #[test]
    fn direct_invocation_with_two_params() {
        assert_eq!(ev("LAMBDA(a,b,a+b)(2,3)"), Some("5".to_string()));
    }

    #[test]
    fn invocation_with_nested_call() {
        assert_eq!(ev("LAMBDA(x,IF(x>1,10,0))(5)"), Some("10".to_string()));
    }

    #[test]
    fn arity_mismatch_is_error() {
        assert_eq!(ev("LAMBDA(a,b,a+b)(2)"), Some("#ERROR!".to_string()));
    }

    #[test]
    fn not_a_lambda_is_none() {
        assert_eq!(ev("SUM(A1:A3)"), None);
    }
}
