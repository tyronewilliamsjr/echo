use super::handlers;
use crate::AppState;
use crate::middleware::session::require_auth;
use axum::{Router, middleware, routing::post};

pub fn router(state: AppState) -> Router<AppState> {
    let unprotected_routes = Router::new().route("/signup", post(handlers::email_signup));

    let protected_routes = Router::new()
        .route("/logout", post(handlers::logout))
        .layer(middleware::from_fn_with_state(state, require_auth));

    Router::new()
        .merge(unprotected_routes)
        .merge(protected_routes)
}
