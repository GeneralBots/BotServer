pub fn register_create_site_keywords(
    _state: std::sync::Arc<dyn botbasic_types::BasicRuntime>,
    _user: botbasic_types::UserSession,
    _engine: &mut rhai::Engine,
) {
}

#[cfg(feature = "llm")]
mod llm_impl;

#[cfg(feature = "llm")]
pub use llm_impl::create_site_keyword;
