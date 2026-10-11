//! `projects::tests` — split per #1443.

use super::*;



    #[test]
    fn project_kind_round_trip() {
        assert_eq!(ProjectKind::parse("bot"), ProjectKind::Bot);
        assert_eq!(ProjectKind::parse("website"), ProjectKind::Website);
        assert_eq!(ProjectKind::parse("apps"), ProjectKind::Apps);
        // #1291/#1372 — deprecated aliases still resolve via the lenient path.
        assert_eq!(ProjectKind::parse("custom"), ProjectKind::Apps);
        // #1372 — unknown kinds fall back on the lenient path but are
        // rejected by parse_strict (see project_kind_parse_strict below).
        assert_eq!(ProjectKind::parse("bogus"), ProjectKind::Apps);
        assert_eq!(ProjectKind::Bot.as_str(), "bot");
        assert_eq!(ProjectKind::Website.as_str(), "website");
        assert_eq!(ProjectKind::Apps.as_str(), "apps");
    }

    /// #1372 — strict parsing: explicit aliases accepted, unknown rejected.
    #[test]
    fn project_kind_parse_strict() {
        // Canonical kinds.
        assert_eq!(
            ProjectKind::parse_strict("website").unwrap(),
            ProjectKind::Website
        );
        assert_eq!(ProjectKind::parse_strict("bot").unwrap(), ProjectKind::Bot);
        assert_eq!(
            ProjectKind::parse_strict("apps").unwrap(),
            ProjectKind::Apps
        );
        // Website aliases (static/HTMX pages — never a VM).
        for alias in ["web", "site", "static", "html", "htmx"] {
            assert_eq!(
                ProjectKind::parse_strict(alias).unwrap(),
                ProjectKind::Website,
                "alias {alias} must map to website"
            );
        }
        // Custom-app aliases (VM-backed node/python apps).
        for alias in ["app", "custom", "node", "nodejs"] {
            assert_eq!(
                ProjectKind::parse_strict(alias).unwrap(),
                ProjectKind::Apps,
                "alias {alias} must map to apps"
            );
        }
        // Unknown kinds are rejected, not coerced.
        assert!(ProjectKind::parse_strict("bogus").is_err());
        assert!(ProjectKind::parse_strict("").is_err());
        // Case-sensitive by design (DB rows are lowercase).
        assert!(ProjectKind::parse_strict("Website").is_err());
    }

    #[test]
    fn list_query_defaults_are_bounded() {
        let q = ListProjectsQuery {
            branch_id: None,
            project_type: None,
            status: None,
            limit: None,
            offset: None,
            all_branches: false,
        };
        assert_eq!(q.branch_id, None);
        assert_eq!(q.limit.unwrap_or(100).min(500), 100);
        assert_eq!(q.offset.unwrap_or(0).max(0), 0);
        let big = ListProjectsQuery {
            branch_id: None,
            project_type: None,
            status: None,
            limit: Some(9001),
            offset: None,
            all_branches: false,
        };
        assert_eq!(big.limit.unwrap_or(100).min(500), 500);
        let neg = ListProjectsQuery {
            branch_id: None,
            project_type: None,
            status: None,
            limit: None,
            offset: Some(-3),
            all_branches: false,
        };
        assert_eq!(neg.offset.unwrap_or(0).max(0), 0);
    }

