use std::path::PathBuf;

use echo_db;
use echo_db::recordings::{FailureCode, RecordingStatus, UpdateRecording};
use echo_infra::aws::sqs::{ProcessingMessage, ValidationMessage};
use echo_infra::recording_storage::{RecordingStore, StorageError};
use serde::Deserialize;
use tokio::process::Command;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::WorkerError;
use crate::{ValidationState, WorkerState};

const MAX_RECORDING_SECONDS: f64 = 60.0 * 60.0 * 5.0;
const MAX_OBJECT_SIZE: u64 = 500 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct ProbeResult {
    streams: Vec<Stream>,
    format: Format,
}

#[derive(Debug, Deserialize)]
struct Stream {
    codec_name: Option<String>,
    codec_type: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Format {
    format_name: Option<String>,
    duration: Option<String>,
}

fn valid_audio_type(format_name: &str, codec: &str) -> bool {
    let formats: Vec<_> = format_name.split(',').collect();

    let has = |name| formats.contains(&name);

    match codec {
        "opus" => has("webm") || has("matroska") || has("ogg"),
        "aac" => has("mov") || has("mp4") || has("m4a"),
        "mp3" => has("mp3"),

        _ => false,
    }
}

async fn create_file(path: &str) -> std::io::Result<File> {
    let path_buf = PathBuf::from(path);

    if let Some(parent) = path_buf.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::File::create_new(path).await
}

/// Delete all temp files
/// #TODO:  If running multiple instances on same server avoid temp being same folder
async fn cleanup_temp() -> Result<(), std::io::Error> {
    // tokio::fs::remove_file(path).await;
    let mut files = tokio::fs::read_dir("temp").await?;

    while let Some(file) = files.next_entry().await? {
        let path = file.path();
        tokio::fs::remove_file(path).await?;
    }

    Ok(())
}

async fn run_ffprobe(path: &str) -> Result<ProbeResult, WorkerError> {
    let cmd = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file",
            "-format_whitelist",
            "mp4,webm,ogg,mp3,m4a,mov",
            "-show_entries",
            "stream=codec_type,codec_name,sample_rate,channels",
            "-show_entries",
            "format=format_name,duration",
            "-of",
            "json",
            path,
        ])
        .kill_on_drop(true)
        .output();

    let output = match timeout(Duration::from_secs(300), cmd).await {
        Err(_) => Err(WorkerError::FFProbeTimeout),
        Ok(Err(_)) => Err(WorkerError::FFProbeSpawn),
        Ok(Ok(output)) => Ok(output),
    }?;

    if !output.status.success() {
        return Err(WorkerError::InvalidRecording(FailureCode::ProbeRejected));
    }

    Ok(serde_json::from_slice(&output.stdout)?)
}

pub async fn handle_validation(
    state: &ValidationState<impl RecordingStore>,
    path: &str,
    msg: &ValidationMessage,
) -> Result<(), WorkerError> {
    let recording = match echo_db::recordings::set_validate(&state.pool, msg.recording_id).await {
        Ok(None) => {
            let result =
                echo_db::recordings::find_by_server_id(&state.pool, msg.recording_id).await?;

            let Some(result) = result else {
                return Err(WorkerError::MissingRecording);
            };

            match result.status {
                RecordingStatus::Validating => return Err(WorkerError::ValidationInProgress),
                RecordingStatus::Completed
                | RecordingStatus::Validated
                | RecordingStatus::Invalid
                | RecordingStatus::Processing
                | RecordingStatus::Processed
                | RecordingStatus::Abort => {
                    return Err(WorkerError::ValidationHandled);
                }
                _ => return Err(WorkerError::InvalidRecordingState),
            }
        }
        Ok(Some(res)) => Ok(res),
        Err(e) => Err(e),
    }?;

    let metadata = state.storage.get_metadata(recording.id).await?;
    if metadata.content_length == 0 {
        return Err(WorkerError::InvalidRecording(FailureCode::EmptyFile));
    }

    if metadata.content_length as u64 > MAX_OBJECT_SIZE {
        return Err(WorkerError::InvalidRecording(FailureCode::FileTooLarge));
    }

    let stream = state
        .storage
        .get_recording(recording.id)
        .await
        .map_err(|err| match err {
            StorageError::MissingObject => {
                WorkerError::InvalidRecording(FailureCode::MissingObject)
            }
            err => WorkerError::Storage(err),
        })?;

    let mut limited = stream.take(MAX_OBJECT_SIZE + 1);
    let mut file = create_file(path).await?;

    let bytes_written = tokio::io::copy(&mut limited, &mut file).await?;
    if bytes_written > MAX_OBJECT_SIZE {
        return Err(WorkerError::InvalidRecording(FailureCode::FileTooLarge));
    }

    file.flush().await?;
    drop(file);

    if bytes_written != metadata.content_length {
        return Err(WorkerError::InvalidRecording(FailureCode::SizeMismatch));
    }

    let probe = run_ffprobe(&path).await?;

    let audio_streams: Vec<_> = probe
        .streams
        .iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("audio"))
        .collect();

    match audio_streams.len() {
        0 => {
            return Err(WorkerError::InvalidRecording(FailureCode::NoAudioStream));
        }
        1 => {}
        _ => {
            return Err(WorkerError::InvalidRecording(FailureCode::MultiAudioSteams));
        }
    }

    let Some(audio_stream) = audio_streams.first() else {
        return Err(WorkerError::InvalidRecording(FailureCode::NoAudioStream));
    };

    let codec = audio_stream
        .codec_name
        .as_deref()
        .ok_or_else(|| WorkerError::InvalidRecording(FailureCode::MissingCodec))?;

    let format = probe
        .format
        .format_name
        .as_deref()
        .ok_or(WorkerError::InvalidRecording(
            FailureCode::UnsupportedAudioType,
        ))?;

    if !valid_audio_type(&format, &codec) {
        return Err(WorkerError::InvalidRecording(
            FailureCode::UnsupportedAudioType,
        ));
    }

    let duration: f64 = probe
        .format
        .duration
        .as_deref()
        .ok_or(WorkerError::InvalidRecording(FailureCode::MissingDuration))?
        .parse()
        .map_err(|_| WorkerError::InvalidRecording(FailureCode::InvalidDuration))?;

    if !duration.is_finite() || duration <= 0.0 {
        return Err(WorkerError::InvalidRecording(FailureCode::InvalidDuration));
    }

    if duration > MAX_RECORDING_SECONDS {
        return Err(WorkerError::InvalidRecording(FailureCode::RecordingTooLong));
    }

    Ok(())
}

pub async fn listen_validation(
    state: &WorkerState<impl RecordingStore>,
) -> Result<(), WorkerError> {
    // Attempt cleanup
    if let Err(_) = cleanup_temp().await {
        tracing::warn!("Failed cleanup at start time");
    }

    tracing::info!("Listening to Validation Queue");
    loop {
        let Some(message) = state.queues.receive_message().await? else {
            continue;
        };

        let body: ValidationMessage = match serde_json::from_str(message.body().unwrap_or("")) {
            Ok(msg) => msg,
            Err(_) => {
                state.queues.delete_validation(&message).await?;
                tracing::error!(message_id = ?&message.message_id(),
                  "invalid parsing validation message");
                continue;
            }
        };

        let path = format!("temp/{}-{}", body.recording_id, Uuid::new_v4());
        let validate_result = handle_validation(&state.validation, &path, &body).await;

        if let Err(_) = tokio::fs::remove_file(&path).await {
            tracing::error!(path = path, "Failed temp file cleanup");
        }

        match validate_result {
            Ok(_) => {
                echo_db::recordings::update_by_id(
                    &state.validation.pool,
                    body.recording_id,
                    UpdateRecording {
                        upload_id: None,
                        status: Some(RecordingStatus::Validated),
                        failure_code: None,
                    },
                )
                .await?;

                let process_message = ProcessingMessage {
                    recording_id: body.recording_id,
                    version: body.version,
                };
                let msg = serde_json::to_string(&process_message)?;

                state.queues.queue_processing(&msg).await?;
                state.queues.delete_validation(&message).await?;
            }
            Err(err) => match err {
                WorkerError::InvalidRecording(failure_code) => {
                    tracing::error!(
                      message_id = ?message.message_id(),
                      "failed validation, invalid recording"
                    );
                    // Recording is not valid, delete on sqs and mark db as invalid
                    echo_db::recordings::update_by_id(
                        &state.validation.pool,
                        body.recording_id,
                        UpdateRecording {
                            upload_id: None,
                            status: Some(RecordingStatus::Invalid),
                            failure_code: Some(failure_code),
                        },
                    )
                    .await?;

                    state.queues.delete_validation(&message).await?;
                }
                WorkerError::ValidationHandled | WorkerError::MissingRecording => {
                    state.queues.delete_validation(&message).await?;
                }
                WorkerError::ValidationInProgress => {
                    state.queues.delete_validation(&message).await?;
                }
                WorkerError::InvalidRecordingState => {
                    let _ = echo_db::recordings::update_by_id(
                        &state.validation.pool,
                        body.recording_id,
                        UpdateRecording {
                            upload_id: None,
                            status: Some(RecordingStatus::Invalid),
                            failure_code: Some(FailureCode::InvalidState),
                        },
                    )
                    .await;
                    tracing::error!(
                        id = body.recording_id.to_string(),
                        "Invalid recording state"
                    );
                }
                WorkerError::Database(err) => {
                    tracing::error!(
                      ?err,
                      recording_id = %body.recording_id,
                      "database error during validation"
                    )
                }
                _ => {
                    echo_db::recordings::update_by_id(
                        &state.validation.pool,
                        body.recording_id,
                        UpdateRecording {
                            upload_id: None,
                            status: Some(RecordingStatus::Uploaded),
                            failure_code: None,
                        },
                    )
                    .await?;
                }
            },
        }
    }
}
