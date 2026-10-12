    use super::*;

    #[test]
    fn test_parse_accept_language_simple() {
        let result = parse_accept_language("en-US");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "en-US");
        assert!((result[0].1 - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_parse_accept_language_with_quality() {
        let result = parse_accept_language("pt-BR,pt;q=0.9,en;q=0.8");
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "pt-BR");
        assert_eq!(result[1].0, "pt");
        assert_eq!(result[2].0, "en");
    }

    #[test]
    fn test_parse_accept_language_sorted_by_quality() {
        let result = parse_accept_language("en;q=0.5,pt-BR;q=0.9,es;q=0.7");
        assert_eq!(result[0].0, "pt-BR");
        assert_eq!(result[1].0, "es");
        assert_eq!(result[2].0, "en");
    }

    #[test]
    fn test_negotiate_locale_exact_match() {
        let requested = vec![("pt-BR".to_string(), 1.0)];
        let result = negotiate_locale(&requested);
        assert!(result.is_some());
        assert_eq!(
            result.as_ref().map(|l| l.to_bcp47()),
            Some("pt-BR".to_string())
        );
    }

    #[test]
    fn test_negotiate_locale_language_match() {
        let requested = vec![("pt-PT".to_string(), 1.0)];
        let result = negotiate_locale(&requested);
        assert!(result.is_some());
        assert_eq!(result.as_ref().map(|l| l.language()), Some("pt"));
    }

    #[test]
    fn test_negotiate_locale_fallback() {
        // A language outside AVAILABLE_LOCALES falls back to the default.
        let requested = vec![("sv".to_string(), 1.0)];
        let result = negotiate_locale(&requested);
        assert!(result.is_some());
        assert_eq!(result.as_ref().map(|l| l.language()), Some("en"));

        // A supported language is honoured rather than falling back.
        let requested = vec![("ja".to_string(), 1.0)];
        assert_eq!(
            negotiate_locale(&requested).as_ref().map(|l| l.language()),
            Some("ja")
        );
    }

    #[test]
    fn test_locale_default() {
        let locale = Locale::default();
        assert_eq!(locale.language(), "en");
        assert_eq!(locale.region(), None);
    }

    #[test]
    fn test_locale_display() {
        let locale = Locale::new("pt-BR").unwrap();
        assert_eq!(locale.to_string(), "pt-BR");
    }

    #[test]
    fn test_localized_error_not_found() {
        let locale = Locale::default();
        let error = LocalizedError::not_found(&locale, "User");
        assert_eq!(error.code, "error-http-404");
    }

    #[test]
    fn test_localized_error_with_details() {
        let locale = Locale::default();
        let error =
            LocalizedError::internal(&locale).with_details(serde_json::json!({"trace_id": "abc123"}));
        assert!(error.details.is_some());
    }

    #[test]
    fn test_available_locales_without_init() {
        let locales = available_locales();
        assert!(!locales.is_empty());
    }
