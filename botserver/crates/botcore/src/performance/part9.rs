use super::*;

impl Default for PaginationConfig {
    fn default() -> Self {
        Self {
            default_page_size: 20,
            max_page_size: 100,
            use_cursor_pagination: true,
            include_total_count: false,
        }
    }
}
