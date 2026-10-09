use super::*;

pub(super) fn installs() -> Arc<AppInstallStats> {
    INSTALLS
        .get_or_init(|| Arc::new(AppInstallStats::default()))
        .clone()
}
pub fn register<S: Clone + Send + Sync + 'static>(r: Router<S>) -> Router<S> {
    let router = r
        .merge(botuifragments::register(Router::new()))
        .route("/api/apps/catalog", get(catalog_handler))
        .route("/api/apps/install", post(install_handler))
        .route("/api/apps/install/stats", get(install_stats_handler))
        .route("/api/agent/plan", post(agent_plan_handler));
    integration_catalog::register(router)
}
