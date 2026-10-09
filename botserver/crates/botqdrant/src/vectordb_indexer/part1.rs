use super::*;

impl UserWorkspace {
    pub(super) fn new(root: PathBuf, bot_id: &Uuid, user_id: &Uuid) -> Self {
        Self {
            root,
            bot_id: *bot_id,
            user_id: *user_id,
        }
    }

    pub(super) fn get_path(&self) -> PathBuf {
        self.root
            .join(self.bot_id.to_string())
            .join(self.user_id.to_string())
    }

    pub(super) fn drive_vectordb(&self) -> String {
        format!("drive_{}_{}", self.bot_id, self.user_id)
    }
}
