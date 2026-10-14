use std::sync::Arc;

use botlib::traits::DriveRepository;

pub use botlib::db_pool::DbPool;
use crate::Document;

/// Per-document undo/redo snapshot stacks (session-lifetime, capped).
#[derive(Default)]
pub struct DocHistory {
    current: Option<Document>,
    undo: std::collections::VecDeque<Document>,
    redo: std::collections::VecDeque<Document>,
}

const DOC_HISTORY_CAP: usize = 20;

impl DocHistory {
    /// Called on every save: archives the previous version.
    pub fn capture(&mut self, doc: Document) {
        let unchanged = self
            .current
            .as_ref()
            .map(|c| c.id == doc.id && c.content == doc.content)
            .unwrap_or(false);
        if unchanged {
            self.current = Some(doc);
            return;
        }
        if let Some(previous) = self.current.take() {
            if self.undo.len() == DOC_HISTORY_CAP {
                self.undo.pop_front();
            }
            self.undo.push_back(previous);
            self.redo.clear();
        }
        self.current = Some(doc);
    }

    pub fn undo(&mut self) -> Option<Document> {
        let previous = self.undo.pop_back()?;
        let restored = self.current.replace(previous.clone());
        if let Some(current) = restored {
            if self.redo.len() == DOC_HISTORY_CAP {
                self.redo.pop_front();
            }
            self.redo.push_back(current);
        }
        self.current.clone()
    }

    pub fn redo(&mut self) -> Option<Document> {
        let next = self.redo.pop_back()?;
        let restored = self.current.replace(next.clone());
        if let Some(current) = restored {
            if self.undo.len() == DOC_HISTORY_CAP {
                self.undo.pop_front();
            }
            self.undo.push_back(current);
        }
        self.current.clone()
    }
}

pub struct DocState {
    pub pool: Arc<DbPool>,
    pub drive: Arc<dyn DriveRepository>,
    pub bucket_name: String,
    /// History keyed by document id; added for server-side undo/redo
    /// (#1138). Uses interior mutability so fragment handlers can share it.
    pub history: tokio::sync::Mutex<std::collections::HashMap<String, DocHistory>>,
}

#[cfg(test)]
mod history_tests {
    use super::*;
    use crate::types_core::Document;

    fn doc(content: &str) -> Document {
        crate::handlers_api::history::test_document(content)
    }

    #[test]
    fn undo_redo_round_trip() {
        let mut h = DocHistory::default();
        h.capture(doc("v1"));
        h.capture(doc("v2"));
        assert_eq!(h.undo().unwrap().content, "v1");
        assert_eq!(h.redo().unwrap().content, "v2");
        assert!(h.redo().is_none());
    }

    #[test]
    fn unchanged_capture_is_noop() {
        let mut h = DocHistory::default();
        h.capture(doc("v1"));
        h.capture(doc("v1"));
        assert!(h.undo().is_none());
    }

    #[test]
    fn undo_clears_redo_branch() {
        let mut h = DocHistory::default();
        h.capture(doc("v1"));
        h.capture(doc("v2"));
        assert_eq!(h.undo().unwrap().content, "v1");
        h.capture(doc("v3")); // branch: redo stack must clear
        assert!(h.redo().is_none());
        assert_eq!(h.undo().unwrap().content, "v1");
    }
}
