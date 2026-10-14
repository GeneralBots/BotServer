pub mod collaboration;
pub mod handlers;
pub mod ooxml;
pub mod pptx;
pub mod routes;
pub mod storage;
pub mod types;
pub mod ui;
pub mod ui_fragments;
pub mod utils;

use std::sync::Arc;

pub use collaboration::{
    handle_get_collaborators, handle_get_mentions, handle_get_presence, handle_get_selections,
    handle_get_typing, handle_slides_websocket,
};
pub use handlers::{
    handle_add_element, handle_add_media, handle_add_slide, handle_apply_theme,
    handle_apply_transition_to_all, handle_delete_element, handle_delete_media,
    handle_delete_presentation, handle_delete_slide, handle_duplicate_slide, handle_end_presenter,
    handle_export_presentation, handle_get_presentation_by_id, handle_get_presenter_notes,
    handle_import_presentation, handle_list_cursors, handle_list_media,
    handle_list_presentations, handle_list_selections, handle_load_presentation,
    handle_new_presentation, handle_remove_transition, handle_reorder_slides,
    handle_save_presentation, handle_search_presentations, handle_set_transition,
    handle_slides_ai, handle_start_presenter, handle_update_cursor, handle_update_element,
    handle_update_media, handle_update_presenter, handle_update_selection,
    handle_update_slide_notes,
};
pub use routes::configure_slides_routes;
pub use storage::DriveOps;
pub use types::{
    Animation, ChartData, ChartDataset, Collaborator, CollaborationCursor,
    CollaborationSelection, ElementContent, ElementStyle, GradientStop, GradientStyle,
    MediaElement, Presentation, PresentationMetadata, PresentationTheme, PresenterSession,
    PresenterViewSettings, SaveResponse, ShadowStyle, Slide, SlideBackground, SlideElement,
    SlideMessage, SlideTransition, TableCell, TableData, ThemeColors, ThemeFonts,
    TransitionConfig, TransitionSound,
};

pub use botlib::db_pool::DbPool;

pub type GetDefaultBotFn = Arc<dyn Fn(&mut diesel::PgConnection) -> (uuid::Uuid, String) + Send + Sync>;

pub struct SlidesState<D: DriveOps> {
    pub drive: Option<D>,
    pub conn: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
}
