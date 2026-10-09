//! Learn UI router wiring.
use axum::{routing::get, Router};

use super::{handle_learn_course_page, handle_learn_create_page, handle_learn_list_page};

pub fn configure_learn_ui_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/suite/learn", get(handle_learn_list_page))
        .route("/suite/learn/create", get(handle_learn_create_page))
        .route("/suite/learn/:id", get(handle_learn_course_page))
}
