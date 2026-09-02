use super::handlers;
use crate::AppState;
use axum::{Router, routing::post};

pub fn router() -> Router<AppState> {
    Router::new().route("/signup", post(handlers::signup))
}
