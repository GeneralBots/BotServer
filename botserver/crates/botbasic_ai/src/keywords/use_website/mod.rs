pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{debug, info, trace};
pub(crate) use rhai::{Dynamic, Engine};

enum WebsiteCrawlStatus {
    NotRegistered,
    Pending,
    Crawled,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_sanitization() {
        assert_eq!(
            sanitize_url_for_collection("https://docs.example.com/path"),
            "docs_example_com_path"
        );
        // The port is part of the sanitised name: it must survive the
        // replacement of `/`, `:` and `.` by `_`.
        assert_eq!(
            sanitize_url_for_collection("http://test.site:9000"),
            "test_site_9000"
        );
    }
}

use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
