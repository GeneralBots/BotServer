pub fn rule_score(severity: &str) -> i32 {
    match severity {
        "critical" => 40,
        "high" => 25,
        "medium" => 15,
        "low" => 5,
        _ => 10,
    }
}

pub fn classify(score: i32, blocked: bool) -> (String, String) {
    if blocked || score >= 80 {
        ("critical".into(), "block".into())
    } else if score >= 60 {
        ("high".into(), "review".into())
    } else if score >= 30 {
        ("medium".into(), "flag".into())
    } else {
        ("low".into(), "allow".into())
    }
}
