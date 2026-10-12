use crate::{app_state::AppState, aws, error::ApiError, middleware::session::CurrentUser};

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use echo_db::recordings::{CreateRecording, Recording, UpdateRecording};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MAX_S3_PART_NUMBER: u32 = 10_000;

const ACCEPTED_MIME_TYPE: [&str; 6] = [
    "audio/webm;codecs=opus",
    "audio/mp4;codecs=mp4a.40.2",
    "audio/mp4",
    "audio/ogg;codecs=opus",
    "audio/webm",
    "audio/ogg",
];

#[derive(Debug, Deserialize)]
pub struct CreateRecordingRequest {
    client_id: Uuid,
    mime_type: String,
}

#[derive(Debug, Serialize)]
pub struct CreateRecordingResponse {
    server_id: Uuid,
    client_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UploadPartRequest {
    part_numbers: Vec<u32>,
}

#[derive(Debug, Serialize)]
pub struct UploadPartResponse {
    upload_urls: Vec<aws::UploadUrl>,
}

#[derive(Debug, Serialize)]
pub struct UploadStateResponse {
    id: Uuid,
    parts: Vec<aws::UploadedPart>,
}

// Extract into a Struct (Recommended)
#[derive(Deserialize, Debug)]
pub struct UploadStateSearchParam {
    client_id: Uuid,
    recovery_id: Uuid,
}

fn generate_key(id: Uuid) -> String {
    format!("recording/{id}")
}

fn user_owns_recording(recording: &Recording, user_id: Uuid) -> Result<(), ApiError> {
    if (recording.user_id != user_id) {
        return Err(ApiError::Unauthorized);
    }

    Ok(())
}

pub async fn init_recording(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Json(request): Json<CreateRecordingRequest>,
) -> Result<(StatusCode, Json<CreateRecordingResponse>), ApiError> {
    // Verify media type is expected value
    if !ACCEPTED_MIME_TYPE.contains(&request.mime_type.as_ref()) {
        return Err(ApiError::invalid_input("Unsupport mimetype"));
    }

    let recording = echo_db::recordings::create(
        &state.pool,
        CreateRecording {
            client_id: request.client_id,
            user_id: user.id,
        },
    )
    .await?;

    let key = generate_key(recording.id);
    let upload_id = aws::init_multi_upload(&key, &request.mime_type).await?;

    let recording = echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        UpdateRecording {
            upload_id: Some(upload_id.to_owned()),
            status: Some(String::from("uploading")),
        },
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(CreateRecordingResponse {
            server_id: recording.id,
            client_id: recording.client_id,
        }),
    ))
}

pub async fn create_upload_parts(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Path(r_id): Path<Uuid>,
    Json(request): Json<UploadPartRequest>,
) -> Result<(StatusCode, Json<UploadPartResponse>), ApiError> {
    let recording = echo_db::recordings::get_by_server_id(&state.pool, r_id).await?;

    user_owns_recording(&recording, user.id)?;

    // Aws limit for parts is 10000
    // https://docs.aws.amazon.com/AmazonS3/latest/userguide/qfacts.html
    if request.part_numbers.is_empty() {
        return Err(ApiError::invalid_input(String::from("Empty parts number")));
    }

    if request
        .part_numbers
        .iter()
        .any(|&n| !(1..=MAX_S3_PART_NUMBER).contains(&n))
    {
        return Err(ApiError::invalid_input("Invalid parts number".to_owned()));
    }

    if recording.status != "uploading" {
        return Err(ApiError::invalid_input(
            "Recording does not have upload status".to_owned(),
        ));
    }

    // #NOTE Checking duplicates, might not be needed if not pref???
    let mut sorted_part_numbers = request.part_numbers.clone();
    sorted_part_numbers.sort_unstable();

    if sorted_part_numbers.windows(2).any(|w| w[0] == w[1]) {
        return Err(ApiError::InvalidInput(
            "Part list has duplicates".to_owned(),
        ));
    }

    let upload_id = recording
        .upload_id
        .as_ref()
        .ok_or(ApiError::MissingUploadId)?;
    let key = generate_key(recording.id);
    let part_numbers: Vec<i32> = request.part_numbers.iter().map(|&n| n as i32).collect();
    let upload_urls = aws::get_upload_url_parts(upload_id, &key, part_numbers).await?;

    Ok((StatusCode::OK, Json(UploadPartResponse { upload_urls })))
}

pub async fn get_upload_state(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Query(params): Query<UploadStateSearchParam>,
) -> Result<(StatusCode, Json<UploadStateResponse>), ApiError> {
    let recording = echo_db::recordings::set_recovery(
        &state.pool,
        params.client_id,
        params.recovery_id,
        user.id,
    )
    .await?;

    let Some(recording) = recording else {
        let local =
            echo_db::recordings::find_by_client_id(&state.pool, params.client_id, user.id).await?;

        // TODO: Properly handle completed states from actively recovering
        return match local {
            Some(_) => Err(ApiError::Conflict),
            None => Err(ApiError::MissingResource("Recording not found".to_owned())),
        };
    };

    // TODO: Handle recovering when no uploadid was set
    if recording.upload_id.is_none() {
        return Err(ApiError::MissingResource("recordin".to_owned()));
    }

    let upload_id = recording
        .upload_id
        .as_ref()
        .ok_or(ApiError::MissingUploadId)?;
    let key = generate_key(recording.id);
    let list_parts = aws::get_list_parts(&key, upload_id).await?;

    Ok((
        StatusCode::OK,
        Json(UploadStateResponse {
            id: recording.id,
            parts: list_parts,
        }),
    ))
}

pub async fn cancel_upload(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Path(r_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let recording = echo_db::recordings::get_by_server_id(&state.pool, r_id).await?;
    user_owns_recording(&recording, user.id)?;

    let upload_id = recording
        .upload_id
        .as_ref()
        .ok_or(ApiError::MissingUploadId)?;
    let key = generate_key(recording.id);

    aws::abort_multiupload(upload_id, &key).await?;

    // upload the recording to status canceled
    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        UpdateRecording {
            upload_id: None,
            status: Some("abort".to_owned()),
        },
    )
    .await?;

    Ok(StatusCode::OK)
}

pub async fn complete_upload(
    Extension(user): Extension<CurrentUser>,
    State(state): State<AppState>,
    Path(r_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let recording = echo_db::recordings::get_by_server_id(&state.pool, r_id).await?;
    user_owns_recording(&recording, user.id)?;

    let upload_id = recording
        .upload_id
        .as_ref()
        .ok_or(ApiError::MissingUploadId)?;
    let key = generate_key(recording.id);

    aws::complete_multi_upload(upload_id, &key).await?;

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        UpdateRecording {
            upload_id: None,
            status: Some("completed".to_owned()),
        },
    )
    .await?;

    Ok(StatusCode::OK)
}
