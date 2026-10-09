use super::*;

pub fn configure_terminal_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(ApiUrls::TERMINAL_WS, get(terminal_ws))
        .route(ApiUrls::TERMINAL_LIST, get(list_terminals))
        .route(ApiUrls::TERMINAL_CREATE, post(create_terminal))
        .route(ApiUrls::TERMINAL_KILL, post(kill_terminal))
}
