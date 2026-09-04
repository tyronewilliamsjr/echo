use super::password::hash_password;
use crate::{
    ApiError, AppState, auth::password::verify_password, middleware::session::CurrentUser,
};
use axum::{Extension, Json, extract::State, http::StatusCode};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use echo_db::{self, sessions::SessionCreate};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SignupResponse {
    pub id: Uuid,
    pub email: String,
}
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub struct SessionToken {
    pub token: String,
    pub hash: [u8; 32],
}

fn generate_session_token() -> SessionToken {
    let mut token_bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut token_bytes);
    let token = URL_SAFE_NO_PAD.encode(token_bytes);

    let hash: [u8; 32] = Sha256::digest(&token_bytes).into();

    SessionToken { token, hash }
}

fn generate_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build(("session", token))
        .http_only(true)
        .secure(secure)
        .path("/")
        .build()
}

fn decode_session_cookie(url_token: &str) -> Result<[u8; 32], ApiError> {
    let token_bytes = URL_SAFE_NO_PAD
        .decode(url_token)
        .map_err(|_| ApiError::Unauthorized)?;

    let token_hash: [u8; 32] = Sha256::digest(&token_bytes).into();

    Ok(token_hash)
}

pub async fn email_signup(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(request): Json<SignupRequest>,
) -> Result<(StatusCode, Json<SignupResponse>), ApiError> {
    let session_token = generate_session_token();
    let hash = hash_password(&request.password)?;

    let mut tx = state.pool.begin().await?;
    let user = echo_db::users::create(
        &mut *tx,
        echo_db::users::CreateUser {
            email: request.email,
        },
    )
    .await?;

    echo_db::privacy::create_default(&mut tx, user.id).await?;
    echo_db::password_credential::create(&mut *tx, user.id, hash).await?;
    echo_db::sessions::create(
        &mut *tx,
        SessionCreate {
            user_id: user.id,
            token_hash: session_token.hash.to_vec(),
            expires_at: Utc::now() + Duration::days(1),
        },
    )
    .await?;

    tx.commit().await?;
    let cookie = generate_cookie(session_token.token, state.cookie_secure);
    cookies.add(cookie);

    Ok((
        StatusCode::CREATED,
        Json(SignupResponse {
            id: user.id,
            email: user.email,
        }),
    ))
}

pub async fn password_login(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(request): Json<LoginRequest>,
) -> Result<StatusCode, ApiError> {
    let user = echo_db::users::find_by_email(&state.pool, request.email)
        .await?
        .ok_or(ApiError::InvalidCredentials)?;

    let pass_cred = echo_db::password_credential::find_by_user(&state.pool, user.id)
        .await?
        .ok_or(ApiError::InvalidCredentials)?;

    verify_password(&request.password, &pass_cred.hash)?;

    let session_token = generate_session_token();
    echo_db::sessions::create(
        &state.pool,
        SessionCreate {
            user_id: user.id,
            token_hash: session_token.hash.to_vec(),
            expires_at: Utc::now() + Duration::days(1),
        },
    )
    .await?;
    let cookie = generate_cookie(session_token.token, state.cookie_secure);
    cookies.add(cookie);

    Ok(StatusCode::OK)
}

pub async fn logout(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<StatusCode, ApiError> {
    if let Some(cookie) = cookies.get("session") {
        let token_hash = decode_session_cookie(cookie.value())?;
        echo_db::sessions::revoke(&state.pool, &token_hash).await?;
    }

    cookies.remove(Cookie::build(("session", "")).path("/").build());
    tracing::info!(user_id = %user.id, "User logged out");

    Ok(StatusCode::OK)
}
