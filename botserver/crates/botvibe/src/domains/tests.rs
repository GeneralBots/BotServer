//! `domains::tests` — split per #1443.

use super::*;



    #[test]
    fn domain_validation_accepts_fqdns() {
        assert_eq!(ProjectDomains::validate_domain("Chat.Example.com"), Ok("chat.example.com".to_string()));
        assert_eq!(ProjectDomains::validate_domain("shop.example.co.uk"), Ok("shop.example.co.uk".to_string()));
    }

    #[test]
    fn domain_validation_rejects_bad_input() {
        assert!(ProjectDomains::validate_domain("").is_err());
        assert!(ProjectDomains::validate_domain("https://x.com").is_err());
        assert!(ProjectDomains::validate_domain("no-dot").is_err());
        assert!(ProjectDomains::validate_domain("bad space.com").is_err());
        assert!(ProjectDomains::validate_domain("bad!chars.com").is_err());
    }

    #[test]
    fn verify_name_prefixes_dns_record() {
        assert_eq!(ProjectDomains::verify_name("app.example.com"), "_gb-verify.app.example.com");
    }

