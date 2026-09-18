use super::handlers;
use crate::{AppState, middleware::session::require_auth};
use axum::{Router, middleware, routing::get, routing::post};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/settings/privacy", get(handlers::get_privacy))
        .route("/settings/privacy", post(handlers::update_privacy))
        .layer(middleware::from_fn_with_state(state, require_auth))
}
