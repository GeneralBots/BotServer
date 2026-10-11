use axum::Router;
use std::sync::Arc;
use botcore::shared::state::AppState;

#[cfg(feature = "deployment")]
pub async fn build_sub_router(
    app_state: &Arc<AppState>,
    port: u16,
    api_router: &mut Router<Arc<AppState>>,
) -> Router<()> {
    let mut sub_router = inner_build_sub_router(app_state, port, api_router).await;
    let dep_pool = app_state.conn.clone();
    let dep_router = crate::deployment::configure_deployment_routes(dep_pool);
    sub_router = sub_router.merge(dep_router);
    sub_router
}

#[cfg(not(feature = "deployment"))]
pub async fn build_sub_router(
    app_state: &Arc<AppState>,
    port: u16,
    api_router: &mut Router<Arc<AppState>>,
) -> Router<()> {
    inner_build_sub_router(app_state, port, api_router).await
}

async fn inner_build_sub_router(
    app_state: &Arc<AppState>,
    port: u16,
    api_router: &mut Router<Arc<AppState>>,
) -> Router<()> {
    let mut sub_router: Router<()> = Router::new();

    super::feature_merge_a::merge_part_a(app_state, port, api_router, &mut sub_router).await;
    super::feature_merge_b::merge_part_b(app_state, api_router, &mut sub_router).await;

    sub_router
}
