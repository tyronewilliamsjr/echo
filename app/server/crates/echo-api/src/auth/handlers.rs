use super::password::hash_password;
use crate::{ApiError, AppState, middleware::session::CurrentUser};
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

pub async fn email_signup(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(request): Json<SignupRequest>,
) -> Result<(StatusCode, Json<SignupResponse>), ApiError> {
    let session_token = generate_session_token();
    let hash = hash_password(&request.password)?;

    println!("Starting db");

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

    println!("finished database calls");
    let cookie = Cookie::build(("session", session_token.token))
        .http_only(true)
        .secure(true)
        .path("/")
        .build();
    cookies.add(cookie);

    Ok((
        StatusCode::CREATED,
        Json(SignupResponse {
            id: user.id,
            email: user.email,
        }),
    ))
}

pub async fn logout(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<StatusCode, ApiError> {
    println!("{:?}", user);

    Ok(StatusCode::OK)
}
