use super::*;

impl ComponentEntry {
    pub(super) fn effective_url(&self) -> &str {
        if cfg!(target_os = "windows") {
            self.url_win.as_deref().unwrap_or(&self.url)
        } else {
            &self.url
        }
    }
}
