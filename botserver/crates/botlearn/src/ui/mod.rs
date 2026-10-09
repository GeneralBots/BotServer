//! Learn suite UI pages.
//!
//! Split from the former monolithic `ui.rs`. The public surface is unchanged:
//! `botlearn::ui::{handle_learn_list_page, handle_learn_course_page,
//! handle_learn_create_page, configure_learn_ui_routes}` all still resolve.
pub mod course_page;
pub mod create_page;
pub mod list_page;
pub mod routes;

pub use course_page::*;
pub use create_page::*;
pub use list_page::*;
pub use routes::*;
