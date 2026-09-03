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
