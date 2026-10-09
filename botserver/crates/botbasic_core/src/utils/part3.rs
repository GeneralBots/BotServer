use super::*;

impl BotDriveLocation {
    /// Full S3 key for a Drive-relative path such as `media/2026/10/a.mp4`.
    pub fn key_for(&self, path: &str) -> String {
        format!("{}{}", self.key_prefix, path.trim_start_matches('/'))
    }

    /// Bucket plus the bot's Drive directory, i.e. what a LIST must target to
    /// enumerate this bot's files.
    pub fn drive_prefix(&self) -> String {
        self.key_prefix.clone()
    }

    /// Exact prefix of this bot's whole tree (`{…}{bot}.gb`): every
    /// `{bot}.gb*` directory of the bot, and nothing of a sibling. Safe to
    /// scan or delete — `cristo.gb` does not match `cristo-test.gbdrive/…`.
    pub fn bot_tree_prefix(&self, bot_name: &str) -> String {
        format!("{}{bot_name}.gb", self.bot_prefix)
    }
}
/// Resolves the bucket and key prefixes for a bot's Drive tree.
///
/// `branch_slug` and `org_slug` must be the `branches.slug` and
/// `organizations.slug` columns, never the UUIDs. The branch decides the
/// workspace prefix inside the org bucket (`{branch}.gbai/`) — a bot whose
/// branch differs from its name (`oppbot` in branch `opportunity-oppbot`)
/// would otherwise write outside its own workspace. It falls back to the bot
/// name only when the caller has no branch information, which is correct for
/// every single-bot branch (`beiner`, `cristo`, …).
pub fn resolve_bot_drive_location(
    bot_name: &str,
    branch_slug: Option<&str>,
    org_slug: Option<&str>,
) -> BotDriveLocation {
    match org_slug.map(str::trim).filter(|s| !s.is_empty()) {
        Some(slug) => {
            let workspace = branch_slug
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(bot_name);
            BotDriveLocation {
                bucket: org_drive_bucket(slug),
                key_prefix: format!("{workspace}.gbai/{bot_name}.gbdrive/"),
                bot_prefix: format!("{workspace}.gbai/"),
            }
        }
        None => BotDriveLocation {
            bucket: format!("{bot_name}.gbai"),
            key_prefix: format!("{bot_name}.gbdrive/"),
            bot_prefix: String::new(),
        },
    }
}
