use std::io::Error;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sqlx;

use crate::auth::password::PasswordError;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Password(#[from] PasswordError),

    #[error("unauthorized")]
    Unauthorized,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Database(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            Self::Password(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            Self::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Unauthorized request").into_response()
            }
        }
    }
}
