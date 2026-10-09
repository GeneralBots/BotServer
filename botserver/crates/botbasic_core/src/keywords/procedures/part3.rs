use super::*;

pub fn preprocess_calls(input: &str) -> String {
    let mut result = String::new();
    let lines: Vec<&str> = input.lines().collect();

    for line in lines {
        let trimmed = line.trim();
        let upper_line = trimmed.to_uppercase();

        if upper_line.starts_with("CALL ") {
            let rest = trimmed[5..].trim();
            let (proc_name, args) = if let Some(paren_start) = rest.find('(') {
                let name = rest[..paren_start].trim().to_uppercase();
                let args_str = if let Some(paren_end) = rest.find(')') {
                    rest[paren_start + 1..paren_end].to_string()
                } else {
                    String::new()
                };
                (name, args_str)
            } else {
                (rest.to_uppercase(), String::new())
            };

            let procedures = PROCEDURES.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(proc) = procedures.get(&proc_name) {
                let arg_values: Vec<&str> = if args.is_empty() {
                    Vec::new()
                } else {
                    args.split(',').map(|a| a.trim()).collect()
                };

                result.push_str("// Begin inlined CALL ");
                result.push_str(&proc_name);
                result.push('\n');

                for (i, param) in proc.params.iter().enumerate() {
                    if i < arg_values.len() && !arg_values[i].is_empty() {
                        use std::fmt::Write;
                        let _ = writeln!(result, "let {} = {};", param, arg_values[i]);
                    }
                }

                result.push_str(&proc.body);
                result.push_str("// End inlined CALL ");
                result.push_str(&proc_name);
            } else {
                result.push_str(line);
            }
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    result
}
pub fn preprocess_procedures(input: &str) -> String {
    let after_subs = preprocess_subs(input);

    let after_functions = preprocess_functions(&after_subs);

    preprocess_calls(&after_functions)
}
pub fn clear_procedures() {
    PROCEDURES.lock().unwrap_or_else(|p| p.into_inner()).clear();
}
pub fn get_procedure_names() -> Vec<String> {
    PROCEDURES
                .lock()
                .unwrap_or_else(|p| p.into_inner())
        .keys()
        .cloned()
        .collect()
}
pub fn has_procedure(name: &str) -> bool {
    PROCEDURES
                .lock()
                .unwrap_or_else(|p| p.into_inner())
        .contains_key(&name.to_uppercase())
}
pub fn get_procedure(name: &str) -> Option<ProcedureDefinition> {
    PROCEDURES
                .lock()
                .unwrap_or_else(|p| p.into_inner())
        .get(&name.to_uppercase())
        .cloned()
}
