use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Database(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}
