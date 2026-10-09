use super::*;

impl BotInfo {
    /// Drive bucket holding this bot's files.
    pub fn bucket_name(&self) -> String {
        self.drive.bucket.clone()
    }

    /// Key prefix for the bot's scripts. Not under `.gbdrive`, so this uses
    /// `bot_prefix` rather than the Drive-only `key_prefix`.
    pub fn dialog_folder(&self) -> String {
        format!("{}{}.gbdialog", self.drive.bot_prefix, self.name)
    }
}
