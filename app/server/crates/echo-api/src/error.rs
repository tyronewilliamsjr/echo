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

    #[error("invalid username or password")]
    InvalidCredentials,

    #[error("unauthorized")]
    Unauthorized,

    #[error("privacy policy missing")]
    MissingPrivacySetting,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Database(error) => {
                tracing::error!(error = ?error, "Database Error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            Self::Password(error) => match error {
                PasswordError::Hash(err) => {
                    tracing::error!(error = ?err, "Password hashing failed");
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
                }
                PasswordError::PHC(err) => {
                    tracing::error!(error = ?err, "Password hashing failed");
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
                }
                PasswordError::InvalidPassword => {
                    (StatusCode::UNAUTHORIZED, "Invalid password").into_response()
                }
            },
            Self::Unauthorized => {
                (StatusCode::UNAUTHORIZED, "Unauthorized request").into_response()
            }
            Self::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid username or password").into_response()
            }
            Self::MissingPrivacySetting => {
                tracing::error!("Missing privacy policy");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Missing privacy setting for user",
                )
                    .into_response()
            }
        }
    }
}
