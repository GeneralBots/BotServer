use rhai::{Dynamic, Engine};
use chrono::{NaiveDateTime, Timelike, Datelike};
use num_format::{Locale, ToFormattedString};
use std::str::FromStr;

pub fn format_keyword(engine: &mut Engine) {
    engine
        .register_custom_syntax(&["FORMAT", "$expr$", "$expr$"], false, {
            move |context, inputs| {
                let value_dyn = context.eval_expression_tree(&inputs[0])?;
                let pattern_dyn = context.eval_expression_tree(&inputs[1])?;

                let value_str = value_dyn.to_string();
                let pattern = pattern_dyn.to_string();

                // --- NUMÉRICO ---
                if let Ok(num) = f64::from_str(&value_str) {
                    let formatted = if pattern.starts_with("N") || pattern.starts_with("C") {
                        // extrai partes: prefixo, casas decimais, locale
                        let (prefix, decimals, locale_tag) = parse_pattern(&pattern);

                        let locale = get_locale(&locale_tag);
                        let symbol = if prefix == "C" {
                            get_currency_symbol(&locale_tag)
                        } else {
                            ""
                        };

                        let int_part = num.trunc() as i64;
                        let frac_part = num.fract();

                        if decimals == 0 {
                            format!("{}{}", symbol, int_part.to_formatted_string(&locale))
                        } else {
                            let frac_scaled =
                                ((frac_part * 10f64.powi(decimals as i32)).round()) as i64;
                            format!(
                                "{}{}.{:0width$}",
                                symbol,
                                int_part.to_formatted_string(&locale),
                                frac_scaled,
                                width = decimals
                            )
                        }
                    } else {
                        match pattern.as_str() {
                            "n" => format!("{:.2}", num),
                            "F" => format!("{:.2}", num),
                            "f" => format!("{}", num),
                            "0%" => format!("{:.0}%", num * 100.0),
                            _ => format!("{}", num),
                        }
                    };

                    return Ok(Dynamic::from(formatted));
                }

                // --- DATA ---
                if let Ok(dt) = NaiveDateTime::parse_from_str(&value_str, "%Y-%m-%d %H:%M:%S") {
                    let formatted = apply_date_format(&dt, &pattern);
                    return Ok(Dynamic::from(formatted));
                }

                // --- TEXTO ---
                let formatted = apply_text_placeholders(&value_str, &pattern);
                Ok(Dynamic::from(formatted))
            }
        })
        .unwrap();
}

// ======================
// Extração de locale + precisão
// ======================
fn parse_pattern(pattern: &str) -> (String, usize, String) {
    let mut prefix = String::new();
    let mut decimals: usize = 2; // padrão 2 casas
    let mut locale_tag = "en".to_string();

    // ex: "C2[pt]" ou "N3[fr]"
    if pattern.starts_with('C') {
        prefix = "C".to_string();
    } else if pattern.starts_with('N') {
        prefix = "N".to_string();
    }

    // procura número após prefixo
    let rest = &pattern[1..];
    let mut num_part = String::new();
    for ch in rest.chars() {
        if ch.is_ascii_digit() {
            num_part.push(ch);
        } else {
            break;
        }
    }
    if !num_part.is_empty() {
        decimals = num_part.parse().unwrap_or(2);
    }

    // procura locale entre colchetes
    if let Some(start) = pattern.find('[') {
        if let Some(end) = pattern.find(']') {
            if end > start {
                locale_tag = pattern[start + 1..end].to_string();
            }
        }
    }

    (prefix, decimals, locale_tag)
}

fn get_locale(tag: &str) -> Locale {
    match tag {
        "en" => Locale::en,
        "fr" => Locale::fr,
        "de" => Locale::de,
        "pt" => Locale::pt,
        "it" => Locale::it,
        "es" => Locale::es,
        _ => Locale::en,
    }
}

fn get_currency_symbol(tag: &str) -> &'static str {
    match tag {
        "en" => "$",
        "pt" => "R$ ",
        "fr" | "de" | "es" | "it" => "€",
        _ => "$",
    }
}

// ==================
// SUPORTE A DATAS
// ==================
fn apply_date_format(dt: &NaiveDateTime, pattern: &str) -> String {
    let mut output = pattern.to_string();

    let year = dt.year();
    let month = dt.month();
    let day = dt.day();
    let hour24 = dt.hour();
    let minute = dt.minute();
    let second = dt.second();
    let millis = dt.and_utc().timestamp_subsec_millis();

    output = output.replace("yyyy", &format!("{:04}", year));
    output = output.replace("yy", &format!("{:02}", year % 100));
    output = output.replace("MM", &format!("{:02}", month));
    output = output.replace("M", &format!("{}", month));
    output = output.replace("dd", &format!("{:02}", day));
    output = output.replace("d", &format!("{}", day));

    output = output.replace("HH", &format!("{:02}", hour24));
    output = output.replace("H", &format!("{}", hour24));

    let mut hour12 = hour24 % 12;
    if hour12 == 0 { hour12 = 12; }
    output = output.replace("hh", &format!("{:02}", hour12));
    output = output.replace("h", &format!("{}", hour12));

    output = output.replace("mm", &format!("{:02}", minute));
    output = output.replace("m", &format!("{}", minute));

    output = output.replace("ss", &format!("{:02}", second));
    output = output.replace("s", &format!("{}", second));

    output = output.replace("fff", &format!("{:03}", millis));

    output = output.replace("tt", if hour24 < 12 { "AM" } else { "PM" });
    output = output.replace("t", if hour24 < 12 { "A" } else { "P" });

    output
}

// ==================
// SUPORTE A TEXTO
// ==================
fn apply_text_placeholders(value: &str, pattern: &str) -> String {
    let mut result = String::new();

    for ch in pattern.chars() {
        match ch {
            '@' => result.push_str(value),
            '&' | '<' => result.push_str(&value.to_lowercase()),
            '>' | '!' => result.push_str(&value.to_uppercase()),
            _ => result.push(ch), // copia qualquer caractere literal
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rhai::Engine;

    fn create_engine() -> Engine {
        let mut engine = Engine::new();
        format_keyword(&mut engine);
        engine
    }

    #[test]
    fn test_numeric_formatting_basic() {
        let engine = create_engine();
        
        // Teste formatação básica
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.567 \"n\"").unwrap(),
            "1234.57"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.5 \"F\"").unwrap(),
            "1234.50"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.567 \"f\"").unwrap(),
            "1234.567"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 0.85 \"0%\"").unwrap(),
            "85%"
        );
    }

    #[test]
    fn test_numeric_formatting_with_locale() {
        let engine = create_engine();
        
        // Teste formatação numérica com locale
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.56 \"N[en]\"").unwrap(),
            "1,234.56"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.56 \"N[pt]\"").unwrap(),
            "1.234,56"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.56 \"N[fr]\"").unwrap(),
            "1 234,56"
        );
    }

    #[test]
    fn test_currency_formatting() {
        let engine = create_engine();
        
        // Teste formatação monetária
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.56 \"C[en]\"").unwrap(),
            "$1,234.56"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.56 \"C[pt]\"").unwrap(),
            "R$ 1.234,56"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.56 \"C[fr]\"").unwrap(),
            "€1 234,56"
        );
    }

    #[test]
    fn test_numeric_decimals_precision() {
        let engine = create_engine();
        
        // Teste precisão decimal
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.5678 \"N0[en]\"").unwrap(),
            "1,235"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.5678 \"N1[en]\"").unwrap(),
            "1,234.6"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.5678 \"N3[en]\"").unwrap(),
            "1,234.568"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT 1234.5 \"C0[en]\"").unwrap(),
            "$1,235"
        );
    }

    #[test]
    fn test_date_formatting() {
        let engine = create_engine();
        
        // Teste formatação de datas
        let result = engine.eval::<String>("FORMAT \"2024-03-15 14:30:25\" \"yyyy-MM-dd HH:mm:ss\"").unwrap();
        assert_eq!(result, "2024-03-15 14:30:25");

        let result = engine.eval::<String>("FORMAT \"2024-03-15 14:30:25\" \"dd/MM/yyyy\"").unwrap();
        assert_eq!(result, "15/03/2024");

        let result = engine.eval::<String>("FORMAT \"2024-03-15 14:30:25\" \"MM/dd/yy\"").unwrap();
        assert_eq!(result, "03/15/24");

        let result = engine.eval::<String>("FORMAT \"2024-03-15 14:30:25\" \"HH:mm\"").unwrap();
        assert_eq!(result, "14:30");
    }

    #[test]
    fn test_date_formatting_12h() {
        let engine = create_engine();
        
        // Teste formato 12h
        let result = engine.eval::<String>("FORMAT \"2024-03-15 14:30:25\" \"hh:mm tt\"").unwrap();
        assert_eq!(result, "02:30 PM");

        let result = engine.eval::<String>("FORMAT \"2024-03-15 09:30:25\" \"hh:mm tt\"").unwrap();
        assert_eq!(result, "09:30 AM");

        let result = engine.eval::<String>("FORMAT \"2024-03-15 00:30:25\" \"h:mm t\"").unwrap();
        assert_eq!(result, "12:30 A");
    }

    #[test]
    fn test_text_formatting() {
        let engine = create_engine();
        
        // Teste formatação de texto
        assert_eq!(
            engine.eval::<String>("FORMAT \"hello\" \"Prefix: @\"").unwrap(),
            "Prefix: hello"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT \"HELLO\" \"Result: &!\"").unwrap(),
            "Result: hello!"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT \"hello\" \"RESULT: >\"").unwrap(),
            "RESULT: HELLO"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT \"Hello\" \"<>\"").unwrap(),
            "hello>"
        );
    }

    #[test]
    fn test_mixed_patterns() {
        let engine = create_engine();
        
        // Teste padrões mistos
        assert_eq!(
            engine.eval::<String>("FORMAT \"hello\" \"@ World!\"").unwrap(),
            "hello World!"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT \"test\" \"< & > ! @\"").unwrap(),
            "test test TEST ! test"
        );
    }

    #[test]
    fn test_edge_cases() {
        let engine = create_engine();
        
        // Teste casos extremos
        assert_eq!(
            engine.eval::<String>("FORMAT 0 \"n\"").unwrap(),
            "0.00"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT -1234.56 \"N[en]\"").unwrap(),
            "-1,234.56"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT \"\" \"@\"").unwrap(),
            ""
        );
        assert_eq!(
            engine.eval::<String>("FORMAT \"test\" \"\"").unwrap(),
            ""
        );
    }

    #[test]
    fn test_invalid_patterns_fallback() {
        let engine = create_engine();
        
        // Teste padrões inválidos (devem fallback para string)
        assert_eq!(
            engine.eval::<String>("FORMAT 123.45 \"invalid\"").unwrap(),
            "123.45"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT \"text\" \"unknown\"").unwrap(),
            "unknown"
        );
    }

    #[test]
    fn test_milliseconds_formatting() {
        let engine = create_engine();
        
        // Teste milissegundos
        let result = engine.eval::<String>("FORMAT \"2024-03-15 14:30:25.123\" \"HH:mm:ss.fff\"").unwrap();
        assert_eq!(result, "14:30:25.123");
    }

    #[test]
    fn test_parse_pattern_function() {
        // Teste direto da função parse_pattern
        assert_eq!(parse_pattern("C[en]"), ("C".to_string(), 2, "en".to_string()));
        assert_eq!(parse_pattern("N3[pt]"), ("N".to_string(), 3, "pt".to_string()));
        assert_eq!(parse_pattern("C0[fr]"), ("C".to_string(), 0, "fr".to_string()));
        assert_eq!(parse_pattern("N"), ("N".to_string(), 2, "en".to_string()));
        assert_eq!(parse_pattern("C2"), ("C".to_string(), 2, "en".to_string()));
    }

    #[test]
    fn test_locale_functions() {
        // Teste funções de locale
        assert!(matches!(get_locale("en"), Locale::en));
        assert!(matches!(get_locale("pt"), Locale::pt));
        assert!(matches!(get_locale("fr"), Locale::fr));
        assert!(matches!(get_locale("invalid"), Locale::en)); // fallback

        assert_eq!(get_currency_symbol("en"), "$");
        assert_eq!(get_currency_symbol("pt"), "R$ ");
        assert_eq!(get_currency_symbol("fr"), "€");
        assert_eq!(get_currency_symbol("invalid"), "$"); // fallback
    }

    #[test]
    fn test_apply_text_placeholders() {
        // Teste direto da função apply_text_placeholders
        assert_eq!(apply_text_placeholders("Hello", "@"), "Hello");
        assert_eq!(apply_text_placeholders("Hello", "&"), "hello");
        assert_eq!(apply_text_placeholders("Hello", ">"), "HELLO");
        assert_eq!(apply_text_placeholders("Hello", "Prefix: @!"), "Prefix: Hello!");
        assert_eq!(apply_text_placeholders("Hello", "<>"), "hello>");
    }

    #[test]
    fn test_expression_parameters() {
        let engine = create_engine();
        
        // Teste com expressões como parâmetros
        assert_eq!(
            engine.eval::<String>("let x = 1000.50; FORMAT x \"N[en]\"").unwrap(),
            "1,000.50"
        );
        assert_eq!(
            engine.eval::<String>("FORMAT (500 + 500) \"n\"").unwrap(),
            "1000.00"
        );
        assert_eq!(
            engine.eval::<String>("let pattern = \"@ World\"; FORMAT \"Hello\" pattern").unwrap(),
            "Hello World"
        );
    }
}