use crate::{ApiError, AppState};
use axum::{Json, extract::State, http::StatusCode};
use echo_db;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct SignupResponse {
    pub id: Uuid,
    pub email: String,
}

pub async fn signup(
    State(state): State<AppState>,
    Json(request): Json<SignupRequest>,
) -> Result<(StatusCode, Json<SignupResponse>), ApiError> {
    let user = echo_db::users::create_user(
        &state.pool,
        echo_db::users::CreateUser {
            email: request.email,
        },
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(SignupResponse {
            id: user.id,
            email: user.email,
        }),
    ))
}
