use crate::{ApiError, AppState, middleware::session::CurrentUser};
use axum::{Extension, Json, extract::State, http::StatusCode};
use echo_db::{self, privacy::CreatePrivacy};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct PrivacyUpdate {
    pub retain_audio: bool,
    pub retain_transcript: bool,
    pub allow_embedding: bool,
    pub allow_reminder: bool,
}

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

pub async fn update_privacy(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Json(request): Json<PrivacyUpdate>,
) -> Result<StatusCode, ApiError> {
    let mut tx = state.pool.begin().await?;

    let latest = echo_db::privacy::remove_current(&mut tx, user.id).await?;
    echo_db::privacy::create(
        &mut tx,
        CreatePrivacy {
            user_id: user.id,
            version: latest.version + 1,
            is_current: true,
            retain_audio: request.retain_audio,
            retain_transcript: request.retain_transcript,
            allow_embedding: request.allow_embedding,
            allow_reminder: request.allow_reminder,
        },
    )
    .await?;

    tx.commit().await?;

    Ok(StatusCode::OK)
}
