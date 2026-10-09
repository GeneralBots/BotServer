pub(crate) use rhai::Dynamic;
pub(crate) use serde_json::Value;
pub(crate) use std::path::PathBuf;

/// Bucket suffix for an org tenant workspace.
pub const GBORG_SUFFIX: &str = ".gborg";

/// Suffix `drive_monitors`' legacy naming appends to a tenant slug when it
/// creates an organization row (`cristo` -> `cristo-org`). It is a database
/// alias, never part of a bucket name.
pub const LEGACY_ORG_SUFFIX: &str = "-org";

/// Where a bot's Drive files physically live.
///
/// A bot that belongs to an org is materialised in the org workspace bucket
/// (`{slug}.gborg`), inside its branch workspace (`{branch}.gbai/`), with the
/// bot directory as an S3 key prefix (`{branch}.gbai/{bot}.gbdrive/…`). A
/// standalone bot owns its `{bot}.gbai` bucket outright. Both shapes are real
/// in prod, so the bucket cannot be derived from the bot name alone — the org
/// and the branch decide.
///
/// `org_slug: None` → standalone layout, `Some(slug)` → org layout.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BotDriveLocation {
    pub bucket: String,
    /// Prefix every Drive key of this bot starts with, including the trailing
    /// separator (empty for the standalone layout, where `{bot}.gbdrive/` is
    /// already the whole key).
    pub key_prefix: String,
    /// Directory prefix of this bot's `{bot}.gb*` folders — not just Drive
    /// files, but also `.gbdialog`, `.gbkb` and `.gbot`. Empty for a
    /// standalone bot (its bucket holds only its own tree), the branch
    /// workspace (`{branch}.gbai/`) inside a shared org bucket. Callers that
    /// seed or delete a single bot must append `{bot}.gb` themselves
    /// (`bot_tree_prefix`) so sibling bots of the same branch survive.
    pub bot_prefix: String,
}

/// Row shape shared by the bot lookups: the bot name plus the two slugs that
/// decide its Drive layout. Both joins are LEFT so a bot whose org or branch
/// row is missing still resolves (to the standalone layout) instead of failing
/// the whole lookup. Raw SQL because the diesel join DSL for `branches` does
/// not type-check against the partial schema in `botbasic_types`.
#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct BotDriveScopeRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    branch_slug: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    org_slug: Option<String>,
}

const BOT_DRIVE_SCOPE_SELECT: &str =
    "SELECT b.name, br.slug AS branch_slug, o.slug AS org_slug \
     FROM bots b \
     LEFT JOIN branches br ON br.id = b.branch_id \
     LEFT JOIN organizations o ON o.org_id = b.org_id";

#[cfg(test)]
mod bot_drive_location_tests {
    use super::*;

    #[test]
    fn standalone_bot_owns_its_gbai_bucket() {
        let loc = resolve_bot_drive_location("beiner", None, None);
        assert_eq!(loc.bucket, "beiner.gbai");
        assert_eq!(loc.key_prefix, "beiner.gbdrive/");
        assert_eq!(loc.key_for("media/2026/10/a.mp4"), "beiner.gbdrive/media/2026/10/a.mp4");
    }

    #[test]
    fn org_bot_uses_gborg_bucket_with_branch_prefix() {
        // Prod shape: org slug `beiner`, branch `beiner`, bot `beiner` ->
        // bucket `beiner.gborg`, key prefix `beiner.gbai/beiner.gbdrive/`.
        let loc = resolve_bot_drive_location("beiner", Some("beiner"), Some("beiner"));
        assert_eq!(loc.bucket, "beiner.gborg");
        assert_eq!(loc.key_prefix, "beiner.gbai/beiner.gbdrive/");
        assert_eq!(
            loc.key_for("media/2026/10/a.mp4"),
            "beiner.gbai/beiner.gbdrive/media/2026/10/a.mp4"
        );
    }

    #[test]
    fn legacy_org_suffix_is_not_part_of_the_bucket_name() {
        // Drive discovery creates orgs as `{tenant}-org` (`ensure_tenant_and_org`),
        // but the physical bucket is `{tenant}.gborg`: cristo-org -> cristo.gborg.
        let loc = resolve_bot_drive_location("cristo", Some("cristo"), Some("cristo-org"));
        assert_eq!(loc.bucket, "cristo.gborg");
        assert_eq!(loc.key_prefix, "cristo.gbai/cristo.gbdrive/");
        assert_eq!(loc.bot_prefix, "cristo.gbai/");
    }

    #[test]
    fn bucket_base_strips_only_a_trailing_legacy_suffix() {
        assert_eq!(org_drive_bucket("cristo-org"), "cristo.gborg");
        assert_eq!(org_drive_bucket("beiner"), "beiner.gborg");
        assert_eq!(org_drive_bucket("default"), "default.gborg");
        assert_eq!(org_drive_bucket("my-organization"), "my-organization.gborg");
        assert_eq!(org_drive_bucket("org-org"), "org.gborg");
    }

    #[test]
    fn branch_decides_the_workspace_prefix_not_the_bot_name() {
        // Prod shape: bot `oppbot` lives in branch `opportunity-oppbot`, whose
        // workspace inside `opportunity-oppbot.gborg` is `opportunity-oppbot.gbai/`.
        let loc = resolve_bot_drive_location(
            "oppbot",
            Some("opportunity-oppbot"),
            Some("opportunity-oppbot-org"),
        );
        assert_eq!(loc.bucket, "opportunity-oppbot.gborg");
        assert_eq!(loc.key_prefix, "opportunity-oppbot.gbai/oppbot.gbdrive/");
        assert_eq!(loc.bot_prefix, "opportunity-oppbot.gbai/");
    }

    #[test]
    fn second_branch_of_an_org_shares_the_org_bucket() {
        // `pragmatismo.gborg` hosts both `pragmatismo.gbai/` and
        // `PragmatismoGB.gbai/` — the org owns the bucket, the branch the workspace.
        let loc = resolve_bot_drive_location(
            "PragmatismoGB",
            Some("PragmatismoGB"),
            Some("pragmatismo"),
        );
        assert_eq!(loc.bucket, "pragmatismo.gborg");
        assert_eq!(loc.key_prefix, "PragmatismoGB.gbai/PragmatismoGB.gbdrive/");
    }

    #[test]
    fn default_org_bot_resolves_to_the_org_workspace() {
        // The instance `default` bot carries org slug `default`; its canonical
        // tree is `default.gborg/default.gbai/…` — the location the archive
        // pass and the htmx seeding in drive_utils already write to.
        let loc = resolve_bot_drive_location("default", Some("default"), Some("default"));
        assert_eq!(loc.bucket, "default.gborg");
        assert_eq!(loc.key_prefix, "default.gbai/default.gbdrive/");
    }

    #[test]
    fn branch_falls_back_to_the_bot_name_when_unknown() {
        let loc = resolve_bot_drive_location("beiner", None, Some("beiner"));
        assert_eq!(loc.key_prefix, "beiner.gbai/beiner.gbdrive/");
        for blank in [Some(""), Some("   ")] {
            let loc = resolve_bot_drive_location("beiner", blank, Some("beiner"));
            assert_eq!(
                loc.key_prefix, "beiner.gbai/beiner.gbdrive/",
                "branch {blank:?} must fall back to the bot name"
            );
        }
    }

    #[test]
    fn org_bot_differs_from_standalone_bot() {
        let standalone = resolve_bot_drive_location("beiner", Some("beiner"), None);
        let org = resolve_bot_drive_location("beiner", Some("beiner"), Some("beiner"));
        assert_ne!(standalone.bucket, org.bucket);
    }

    #[test]
    fn blank_or_missing_slug_falls_back_to_standalone() {
        for slug in [None, Some(""), Some("   ")] {
            let loc = resolve_bot_drive_location("beiner", Some("beiner"), slug);
            assert_eq!(loc.bucket, "beiner.gbai", "slug {slug:?} must not produce .gborg");
        }
    }

    #[test]
    fn key_for_tolerates_leading_slash() {
        let org = resolve_bot_drive_location("beiner", Some("beiner"), Some("beiner"));
        assert_eq!(
            org.key_for("/media/a.mp4"),
            "beiner.gbai/beiner.gbdrive/media/a.mp4"
        );
    }

    #[test]
    fn standalone_bot_prefix_is_empty() {
        // A standalone bucket holds only this bot's tree, so seeding and
        // deleting operate on the bucket root.
        let loc = resolve_bot_drive_location("beiner", None, None);
        assert_eq!(loc.bot_prefix, "");
    }

    #[test]
    fn org_bot_prefix_is_the_branch_workspace() {
        // Sibling bots of one branch share the workspace prefix; the bot name
        // is appended by the callers that seed or delete a bot's tree.
        let loc = resolve_bot_drive_location("beiner", Some("beiner"), Some("beiner"));
        assert_eq!(loc.bot_prefix, "beiner.gbai/");
    }

    #[test]
    fn org_bot_prefix_and_drive_prefix_agree_on_the_bot_tree() {
        // The Drive prefix is the bot prefix plus the Drive directory, so a
        // Drive key is always inside the bot's own tree.
        let loc = resolve_bot_drive_location("beiner", Some("beiner"), Some("beiner"));
        let drive_key = loc.key_for("media/a.mp4");
        assert!(
            drive_key.starts_with(&loc.bot_prefix),
            "{drive_key} must live under {}",
            loc.bot_prefix
        );
    }

    #[test]
    fn sibling_bots_in_one_org_have_disjoint_trees() {
        // Siblings share bucket AND branch workspace, so disjointness is a
        // property of the per-bot tree prefix, not of the workspace prefix.
        let a = resolve_bot_drive_location("bot_a", Some("acme"), Some("acme"));
        let b = resolve_bot_drive_location("bot_b", Some("acme"), Some("acme"));
        assert_eq!(a.bucket, b.bucket, "siblings share the org bucket");
        assert_eq!(a.bot_prefix, b.bot_prefix, "and the branch workspace");
        assert!(
            !b.key_for("x.txt").starts_with(&a.bot_tree_prefix("bot_a")),
            "bot_b's key must not fall inside bot_a's tree"
        );
    }

    #[test]
    fn bot_tree_prefix_does_not_reach_a_sibling_bot() {
        // Deleting `cristo` must not touch `cristo-test`: the tree prefix ends
        // at `cristo.gb`, and `cristo-test.gbdrive/…` does not match it.
        let loc = resolve_bot_drive_location("cristo", Some("cristo"), Some("cristo-org"));
        let tree = loc.bot_tree_prefix("cristo");
        assert_eq!(tree, "cristo.gbai/cristo.gb");
        assert!(
            !format!("cristo.gbai/cristo-test.gbdrive/a.bas").starts_with(&tree),
            "sibling bot must stay outside the delete prefix"
        );
        assert!(
            format!("cristo.gbai/{}", "cristo.gbdialog/x.bas").starts_with(&tree),
            "the bot's own tree must be covered"
        );
    }

    #[test]
    fn standalone_tree_prefix_covers_the_bucket_root_tree() {
        let loc = resolve_bot_drive_location("beiner", None, None);
        assert_eq!(loc.bot_tree_prefix("beiner"), "beiner.gb");
        assert!(loc.key_for("media/a.mp4").starts_with(&loc.bot_tree_prefix("beiner")));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
