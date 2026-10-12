use aws_sdk_s3::presigning::PresigningConfigError;
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

    #[error("invalid recording is missing upload id")]
    MissingUploadId,

    #[error("validation error")]
    InvalidInput(String),

    #[error("invalid username or password")]
    InvalidCredentials,

    #[error("unauthorized")]
    Unauthorized,

    #[error("non existing resource")]
    MissingResource(String),

    #[error("conflict")]
    Conflict,

    #[error("privacy policy missing")]
    MissingPrivacySetting,

    #[error("error occurred during aws operation")]
    AwsError(String),

    #[error(transparent)]
    AwsPresignedConfigError(#[from] PresigningConfigError),

    #[error("failed to generate presigned upload URL")]
    AwsPresignedUpload(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("failed to abort multiupload")]
    AwsAbortMultiUpload(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("failed to initiate multiupload")]
    AwsMultipartUpload(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("failed to complete multiupload")]
    AwsCompleteUpload(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("failed to get list of upload parts")]
    AwsListParts(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl ApiError {
    pub fn invalid_input(message: impl Into<String>) -> Self {
        ApiError::InvalidInput(message.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::AwsError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            Self::AwsPresignedConfigError(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            Self::AwsAbortMultiUpload(err)
            | Self::AwsCompleteUpload(err)
            | Self::AwsListParts(err)
            | Self::AwsPresignedUpload(err)
            | Self::AwsMultipartUpload(err) => {
                tracing::error!(error = err, "Aws response error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            Self::MissingUploadId => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
            Self::Conflict => (StatusCode::CONFLICT, "Conflict").into_response(),
            Self::MissingResource(msg) => {
                (StatusCode::NOT_FOUND, "Missing resource").into_response()
            }
            Self::InvalidInput(msg) => {
                tracing::warn!(
                    error = %msg,
                    "Invalid request input"
                );
                (StatusCode::BAD_REQUEST, "Invalid inputs").into_response()
            }
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
