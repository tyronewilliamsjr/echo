use crate::{ApiError, AppState, middleware::session::CurrentUser};
use axum::{Extension, Json, extract::State, http::StatusCode};
use echo_db;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct PrivacyResponse {
    pub id: Uuid,
    pub version: i32,
    pub retain_audio: bool,
    pub retain_transcript: bool,
    pub allow_embedding: bool,
    pub allow_reminder: bool,
}

pub async fn get_privacy(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<PrivacyResponse>), ApiError> {
    let policy = echo_db::privacy::find_current(&state.pool, user.id)
        .await?
        .ok_or(ApiError::MissingPrivacySetting)?;

    Ok((
        StatusCode::OK,
        Json(PrivacyResponse {
            id: policy.id,
            version: policy.version,
            retain_audio: policy.retain_audio,
            retain_transcript: policy.retain_transcript,
            allow_embedding: policy.allow_embedding,
            allow_reminder: policy.allow_reminder,
        }),
    ))
}
