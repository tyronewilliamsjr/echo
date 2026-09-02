use crate::{AppState, users};
use axum::Router;

pub fn create_app(state: AppState) -> Router {
    Router::new()
        .nest("/users", users::router())
        .with_state(state)
}
