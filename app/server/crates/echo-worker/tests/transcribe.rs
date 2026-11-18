mod common;
use std::sync::Arc;

use echo_db::recordings::{FailureCode, RecordingStatus};
use echo_infra::{aws::sqs::ProcessingMessage, recording_storage::LocalRecordingStorage};
use echo_worker::{
    ProcessingState, error::WorkerError, tempdir::TempDir, transcribe::process_and_persist,
};
use sqlx::PgPool;

use common::{create_recording, create_user};
use uuid::Uuid;

use crate::common::whisper_context;

#[sqlx::test(migrator = "echo_db::MIGRATOR")]
async fn test_valid_webm(pool: PgPool) {
    let file_to_test = "tests/fixtures/valid/valid.webm";
    let test_dir = TempDir::new().await.unwrap();
    let claim_id = Uuid::new_v4();
    let storage_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let local_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();

    let store = LocalRecordingStorage::new(&storage_path.path_buf.clone().into_string().unwrap());

    let state = ProcessingState {
        pool,
        context: whisper_context(),
        storage: Arc::new(store),
    };

    let user = create_user(&state.pool).await.unwrap();
    let recording = create_recording(&state.pool, user.id).await.unwrap();

    state
        .storage
        .copy_local_to_root(recording.id, file_to_test)
        .await
        .unwrap();

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        echo_db::recordings::UpdateRecording {
            upload_id: None,
            status: Some(RecordingStatus::Validated),
            failure_code: None,
        },
    )
    .await
    .unwrap();

    let body = ProcessingMessage {
        version: 1,
        recording_id: recording.id,
    };

    process_and_persist(&state, &local_path, &body, claim_id)
        .await
        .unwrap();

    let updated_recording = echo_db::recordings::get_by_server_id(&state.pool, recording.id)
        .await
        .unwrap();

    echo_db::transcript::get_by_recording_id(&state.pool, updated_recording.id)
        .await
        .unwrap();

    assert!(matches!(
        updated_recording.status,
        RecordingStatus::Processed
    ));
}

#[sqlx::test(migrator = "echo_db::MIGRATOR")]
async fn test_valid_mp3(pool: PgPool) {
    let file_to_test = "tests/fixtures/valid/valid.mp3";
    let test_dir = TempDir::new().await.unwrap();
    let claim_id = Uuid::new_v4();
    let storage_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let local_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();

    let store = LocalRecordingStorage::new(&storage_path.path_buf.clone().into_string().unwrap());

    let state = ProcessingState {
        pool,
        context: whisper_context(),
        storage: Arc::new(store),
    };

    let user = create_user(&state.pool).await.unwrap();
    let recording = create_recording(&state.pool, user.id).await.unwrap();

    state
        .storage
        .copy_local_to_root(recording.id, file_to_test)
        .await
        .unwrap();

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        echo_db::recordings::UpdateRecording {
            upload_id: None,
            status: Some(RecordingStatus::Validated),
            failure_code: None,
        },
    )
    .await
    .unwrap();

    let body = ProcessingMessage {
        version: 1,
        recording_id: recording.id,
    };

    process_and_persist(&state, &local_path, &body, claim_id)
        .await
        .unwrap();

    let updated_recording = echo_db::recordings::get_by_server_id(&state.pool, recording.id)
        .await
        .unwrap();

    echo_db::transcript::get_by_recording_id(&state.pool, updated_recording.id)
        .await
        .unwrap();

    assert!(matches!(
        updated_recording.status,
        RecordingStatus::Processed
    ));
}

#[sqlx::test(migrator = "echo_db::MIGRATOR")]
async fn test_valid_m4a(pool: PgPool) {
    let file_to_test = "tests/fixtures/valid/valid.m4a";
    let test_dir = TempDir::new().await.unwrap();
    let claim_id = Uuid::new_v4();
    let storage_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let local_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();

    let store = LocalRecordingStorage::new(&storage_path.path_buf.clone().into_string().unwrap());

    let state = ProcessingState {
        pool,
        context: whisper_context(),
        storage: Arc::new(store),
    };

    let user = create_user(&state.pool).await.unwrap();
    let recording = create_recording(&state.pool, user.id).await.unwrap();

    state
        .storage
        .copy_local_to_root(recording.id, file_to_test)
        .await
        .unwrap();

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        echo_db::recordings::UpdateRecording {
            upload_id: None,
            status: Some(RecordingStatus::Validated),
            failure_code: None,
        },
    )
    .await
    .unwrap();

    let body = ProcessingMessage {
        version: 1,
        recording_id: recording.id,
    };

    process_and_persist(&state, &local_path, &body, claim_id)
        .await
        .unwrap();

    let updated_recording = echo_db::recordings::get_by_server_id(&state.pool, recording.id)
        .await
        .unwrap();

    echo_db::transcript::get_by_recording_id(&state.pool, updated_recording.id)
        .await
        .unwrap();

    assert!(matches!(
        updated_recording.status,
        RecordingStatus::Processed
    ));
}

#[sqlx::test(migrator = "echo_db::MIGRATOR")]
async fn test_invalid_empty(pool: PgPool) {
    let file_to_test = "tests/fixtures/invalid/empty.webm";
    let test_dir = TempDir::new().await.unwrap();
    let storage_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let local_path = TempDir::from_parent(&test_dir.path_buf).await.unwrap();
    let claim_id = Uuid::new_v4();

    let store = LocalRecordingStorage::new(&storage_path.path_buf.clone().into_string().unwrap());

    let state = ProcessingState {
        pool,
        context: whisper_context(),
        storage: Arc::new(store),
    };

    let user = create_user(&state.pool).await.unwrap();
    let recording = create_recording(&state.pool, user.id).await.unwrap();

    state
        .storage
        .copy_local_to_root(recording.id, file_to_test)
        .await
        .unwrap();

    echo_db::recordings::update_by_id(
        &state.pool,
        recording.id,
        echo_db::recordings::UpdateRecording {
            upload_id: None,
            status: Some(RecordingStatus::Validated),
            failure_code: None,
        },
    )
    .await
    .unwrap();

    let body = ProcessingMessage {
        recording_id: recording.id,
        version: 1,
    };

    let result = process_and_persist(&state, &local_path, &body, claim_id)
        .await
        .unwrap_err();

    println!("{:?}", result);

    assert!(matches!(
        result,
        WorkerError::InvalidRecording(FailureCode::PcmRejected)
    ));
}
