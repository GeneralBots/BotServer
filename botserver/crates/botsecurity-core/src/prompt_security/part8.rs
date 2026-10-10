pub fn quick_injection_check(input: &str) -> bool {
    let suspicious_patterns = [
        "ignore previous",
        "ignore all",
        "disregard",
        "forget your",
        "new instructions",
        "system prompt",
        "reveal your",
        "jailbreak",
        "dan mode",
        "developer mode",
    ];

    let input_lower = input.to_lowercase();
    suspicious_patterns.iter().any(|p| input_lower.contains(p))
}
pub fn escape_for_prompt(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
