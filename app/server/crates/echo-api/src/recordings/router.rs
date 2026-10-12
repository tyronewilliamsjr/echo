use super::handlers;

use crate::{AppState, middleware::session::require_auth};
use axum::{
    Router, middleware,
    routing::{get, post},
};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/init", post(handlers::init_recording))
        .route("/state", get(handlers::get_upload_state))
        .route("/{r_id}/parts", post(handlers::create_upload_parts))
        .route("/{r_id}/complete", post(handlers::complete_upload))
        .route("/{r_id}/cancel", post(handlers::cancel_upload))
        .layer(middleware::from_fn_with_state(state, require_auth))
}
