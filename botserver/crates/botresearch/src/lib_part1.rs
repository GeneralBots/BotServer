use super::*;

pub fn configure_research_routes<S: ResearchState>() -> Router<Arc<S>> {
    Router::new()
        .merge(web_search::configure_web_search_routes::<S>())
        .route(ROUTE_RESEARCH_COLLECTIONS, get(handle_list_collections::<S>))
        .route(
            ROUTE_RESEARCH_COLLECTIONS_NEW,
            post(handle_create_collection::<S>),
        )
        .route(ROUTE_RESEARCH_COLLECTION_BY_ID, get(handle_get_collection::<S>))
        .route(ROUTE_RESEARCH_SEARCH, post(handle_search::<S>))
        .route(ROUTE_RESEARCH_RECENT, get(handle_recent_searches::<S>))
        .route(ROUTE_RESEARCH_TRENDING, get(handle_trending_tags::<S>))
        .route(ROUTE_RESEARCH_PROMPTS, get(handle_prompts::<S>))
        .route(
            ROUTE_RESEARCH_EXPORT_CITATIONS,
            get(handle_export_citations::<S>),
        )
        .route(ROUTE_RESEARCH_SOURCE_COUNTS, get(handle_source_counts::<S>))
        .route(ROUTE_RESEARCH_SOURCES, get(handle_sources::<S>))
        .route(
            ROUTE_RESEARCH_COLLECTIONS_SAVE,
            post(handle_collections_save::<S>),
        )
        .route(ROUTE_RESEARCH_SHARE, post(handle_share::<S>))
}
