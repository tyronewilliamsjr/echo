use crate::{AppState, auth, users};
use axum::Router;
use tower_cookies::CookieManagerLayer;

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .nest("/users", users::router())
        .nest("/auth", auth::router(state.clone()))
        .with_state(state)
        .layer(CookieManagerLayer::new())
}
