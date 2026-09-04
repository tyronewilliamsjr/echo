use crate::{AppState, error::ApiError};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use sha2::{Digest, Sha256};
use tower_cookies::Cookies;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: Uuid,
}

pub async fn require_auth(
    State(state): State<AppState>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let cookie = cookies.get("session").ok_or(ApiError::Unauthorized)?;
    let token_bytes = URL_SAFE_NO_PAD
        .decode(cookie.value())
        .map_err(|_| ApiError::Unauthorized)?;

    let token_hash: [u8; 32] = Sha256::digest(&token_bytes).into();
    let session = echo_db::sessions::find_by_hash(&state.pool, token_hash.to_vec())
        .await?
        .ok_or(ApiError::Unauthorized)?;

    if session.expires_at <= Utc::now() {
        // return Err(ApiError::Unauthorized);
    }

    if session.revoked_at.is_some() {
        // return Err(ApiError::Unauthorized);
    }

    request.extensions_mut().insert(CurrentUser {
        id: session.user_id,
    });

    Ok(next.run(request).await)
}
