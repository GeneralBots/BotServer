//! `api::tests` — split per #1443.

use super::*;
use super::run::derive_project_name;



    #[test]
    fn truncate_chars_never_panics_on_multibyte_boundary() {
        // 79 ASCII chars + a 2-byte 'é' at the boundary would panic under
        // byte slicing; the char-safe cut must not.
        let s = format!("{}é", "a".repeat(79));
        let out = truncate_chars(&s, 80);
        assert!(s.starts_with(out));
        assert!(out.len() <= 80);
        assert_eq!(truncate_chars("short", 80), "short");
        assert_eq!(truncate_chars("", 80), "");
    }

    #[test]
    fn truncate_chars_handles_portuguese_and_emoji() {
        // 'coração' has multi-byte 'ç'/'ã'; '🚀' is a 4-byte emoji. Both must
        // survive a cut that lands inside them without panicking.
        let pt = "eu quero agendar um batizado na catedral da sé, coração".repeat(4);
        assert!(truncate_chars(&pt, 80).len() <= 80);
        let emoji = format!("{}🚀", "x".repeat(79));
        assert!(truncate_chars(&emoji, 80).len() <= 80);
    }

    #[test]
    fn derive_project_name_strips_stopwords_and_slugs() {
        assert_eq!(
            derive_project_name("Create a calculator web app with + - * / buttons"),
            "calculator-web-app"
        );
        assert_eq!(
            derive_project_name("Build a new landing page"),
            "landing-page"
        );
        assert_eq!(derive_project_name("refactor the auth module"), "refactor-auth-module");
        assert_eq!(derive_project_name(""), "app");
        assert_eq!(derive_project_name("   "), "app");
        // #1272 — deictic phrases never become project names.
        assert_eq!(
            derive_project_name("Deploy the selected project to production"),
            "deploy-to-production"
        );
        assert_eq!(
            derive_project_name("Update this project settings"),
            "update-settings"
        );
    }

