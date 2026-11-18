use std::sync::Arc;

use echo_db::{
    recordings::{FailureCode, Recording, UpdateRecording},
    users::{CreateUser, User},
};
use echo_infra::{aws::sqs::ValidationMessage, recording_storage::LocalRecordingStorage};
use echo_worker::{
    ValidationState, error::WorkerError, tempdir::TempDir, validate::handle_validation,
};
use sqlx::PgPool;
use uuid::Uuid;

async fn create_user(pool: &PgPool) -> Result<User, sqlx::error::Error> {
    let email = format!("{}@email.com", Uuid::new_v4());
    echo_db::users::create(pool, CreateUser { email }).await
}

async fn create_recording(pool: &PgPool, user_id: Uuid) -> Result<Recording, sqlx::error::Error> {
    echo_db::recordings::create(
        pool,
        echo_db::recordings::CreateRecording {
            client_id: Uuid::new_v4(),
            user_id,
        },
    )
    .await
}

#[sqlx::test(migrator = "echo_db::MIGRATOR")]
async fn test_valid_mp3(pool: PgPool) {
    let file_to_test = "tests/fixtures/Recording.mp3";
    let test_dir = TempDir::new().await.unwrap();
    let storage_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let local_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();

    let store = LocalRecordingStorage::new(&storage_path.path_buf.clone().into_string().unwrap());
    let state = ValidationState {
        pool,
        storage: Arc::new(store),
    };

    let user = create_user(&state.pool).await.unwrap();
    let recording = create_recording(&state.pool, user.id).await.unwrap();
    let msg = ValidationMessage {
        recording_id: recording.id,
        version: 1,
    };

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        UpdateRecording {
            failure_code: None,
            status: Some(echo_db::recordings::RecordingStatus::Uploaded),
            upload_id: None,
        },
    )
    .await
    .unwrap();

    state
        .storage
        .copy_local_to_root(recording.id, file_to_test)
        .await
        .unwrap();

    let file_path = local_path
        .join(&recording.id.to_string())
        .into_string()
        .unwrap();

    handle_validation(&state, &file_path, &msg).await.unwrap();
}

#[sqlx::test(migrator = "echo_db::MIGRATOR")]
async fn test_valid_m4a(pool: PgPool) {
    let file_to_test = "tests/fixtures/valid/valid.m4a";
    let test_dir = TempDir::new().await.unwrap();
    let storage_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let local_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();

    let store = LocalRecordingStorage::new(&storage_path.path_buf.clone().into_string().unwrap());
    let state = ValidationState {
        pool,
        storage: Arc::new(store),
    };

    let user = create_user(&state.pool).await.unwrap();
    let recording = create_recording(&state.pool, user.id).await.unwrap();
    let msg = ValidationMessage {
        recording_id: recording.id,
        version: 1,
    };

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        UpdateRecording {
            failure_code: None,
            status: Some(echo_db::recordings::RecordingStatus::Uploaded),
            upload_id: None,
        },
    )
    .await
    .unwrap();

    state
        .storage
        .copy_local_to_root(recording.id, file_to_test)
        .await
        .unwrap();

    let file_path = local_path
        .join(&recording.id.to_string())
        .into_string()
        .unwrap();

    handle_validation(&state, &file_path, &msg).await.unwrap();
}

#[sqlx::test(migrator = "echo_db::MIGRATOR")]
async fn test_valid_webm(pool: PgPool) {
    let file_to_test = "tests/fixtures/Recording.webm";
    let test_dir = TempDir::new().await.unwrap();
    let storage_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let local_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();

    let store = LocalRecordingStorage::new(&storage_path.path_buf.clone().into_string().unwrap());
    let state = ValidationState {
        pool,
        storage: Arc::new(store),
    };

    let user = create_user(&state.pool).await.unwrap();
    let recording = create_recording(&state.pool, user.id).await.unwrap();
    let msg = ValidationMessage {
        recording_id: recording.id,
        version: 1,
    };

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        UpdateRecording {
            failure_code: None,
            status: Some(echo_db::recordings::RecordingStatus::Uploaded),
            upload_id: None,
        },
    )
    .await
    .unwrap();

    state
        .storage
        .copy_local_to_root(recording.id, file_to_test)
        .await
        .unwrap();

    let file_path = local_path
        .join(&recording.id.to_string())
        .into_string()
        .unwrap();

    handle_validation(&state, &file_path, &msg).await.unwrap();
}

#[sqlx::test(migrator = "echo_db::MIGRATOR")]
async fn test_invalid_wav(pool: PgPool) {
    let file_to_test = "tests/fixtures/Recording.wav";
    let test_dir = TempDir::new().await.unwrap();
    let storage_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let local_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();

    let store = LocalRecordingStorage::new(&storage_path.path_buf.clone().into_string().unwrap());
    let state = ValidationState {
        pool,
        storage: Arc::new(store),
    };

    let user = create_user(&state.pool).await.unwrap();
    let recording = create_recording(&state.pool, user.id).await.unwrap();
    let msg = ValidationMessage {
        recording_id: recording.id,
        version: 1,
    };

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        UpdateRecording {
            failure_code: None,
            status: Some(echo_db::recordings::RecordingStatus::Uploaded),
            upload_id: None,
        },
    )
    .await
    .unwrap();

    state
        .storage
        .copy_local_to_root(recording.id, file_to_test)
        .await
        .unwrap();

    let file_path = local_path
        .join(&recording.id.to_string())
        .into_string()
        .unwrap();

    let result = handle_validation(&state, &file_path, &msg)
        .await
        .unwrap_err();

    assert!(matches!(
        result,
        WorkerError::InvalidRecording(FailureCode::ProbeRejected)
    ));
}
