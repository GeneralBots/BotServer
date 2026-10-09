use super::*;

#[cfg(feature = "llm")]
#[cfg(not(feature = "vibe"))]
pub fn reexport_create_site_keyword() -> Option<fn(Arc<AppState>, UserSession, &mut Engine)> {
    Some(create_site_keyword)
}
