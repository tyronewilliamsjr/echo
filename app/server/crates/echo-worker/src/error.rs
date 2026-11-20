use echo_db::recordings::FailureCode;
use echo_infra::aws::sqs::QueueError;
use echo_infra::recording_storage::StorageError;

use tokio::{
    io::{self},
    task::JoinError,
};
use whisper_rs::WhisperError;

#[derive(Debug, thiserror::Error)]
pub enum WorkerError {
    #[error("ffprobe could not run")]
    FFProbeSpawn(#[source] std::io::Error),

    #[error("ffmpeg could not run")]
    FFMpegSpawn(#[source] std::io::Error),

    #[error("ffprobe failed during run")]
    FFProbeFailed,

    #[error("ffmpeg failed during run")]
    FFMpegFailed,

    #[error("FFprobe timedout")]
    FFProbeTimeout,

    #[error("FFmpeg timedout")]
    FFMpegTimeout,

    #[error("Io error")]
    IO(#[from] io::Error),

    #[error("Task join error: {0}")]
    TaskJoin(#[from] JoinError),

    #[error("Unable to locate model")]
    ModelMissing,

    #[error("Recording does not exists")]
    MissingRecording,

    #[error("Validation in progress")]
    ValidationInProgress,

    #[error("Validation is handled")]
    ValidationHandled,

    #[error("Processing in progress")]
    ProcessingInProgress,

    #[error("Processing is handled")]
    ProcessingHandled,

    #[error("Recording state is invalid")]
    InvalidRecordingState,

    #[error("Recording state is invalid")]
    UnexpectedRecordingState,

    #[error("Recording is invalid")]
    InvalidRecording(FailureCode),

    #[error(transparent)]
    Whisper(#[from] WhisperError),

    #[error(transparent)]
    Queue(#[from] QueueError),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    CommandParse(#[from] serde_json::Error),
}
