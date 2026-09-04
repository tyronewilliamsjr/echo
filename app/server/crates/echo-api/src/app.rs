use crate::{AppState, auth, users};
use axum::Router;
use tower_cookies::CookieManagerLayer;
use tower_http::trace::TraceLayer;

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .nest("/users", users::router(state.clone()))
        .nest("/auth", auth::router(state.clone()))
        .with_state(state)
        .layer(CookieManagerLayer::new())
        .layer(TraceLayer::new_for_http())
}
