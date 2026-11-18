use std::{path::PathBuf, sync::Arc};

use echo_infra::{aws::sqs::Queue, recording_storage::RecordingStore};
use sqlx::PgPool;
use tokio::fs::File;
use whisper_rs::WhisperContext;

pub mod error;
pub mod tempdir;
pub mod transcribe;
pub mod validate;

pub struct ValidationState<S>
where
    S: RecordingStore,
{
    pub pool: PgPool,
    pub storage: Arc<S>,
}

pub struct ProcessingState<S>
where
    S: RecordingStore,
{
    pub pool: PgPool,
    pub storage: Arc<S>,
    pub context: Arc<WhisperContext>,
}

pub struct WorkerState<S>
where
    S: RecordingStore,
{
    pub validation: ValidationState<S>,
    pub queues: Queue,
}

/// Delete all temp files
pub async fn cleanup_temp() -> Result<(), std::io::Error> {
    // tokio::fs::remove_file(path).await;
    let mut files = tokio::fs::read_dir("temp").await?;

    while let Some(file) = files.next_entry().await? {
        let path = file.path();
        tokio::fs::remove_file(path).await?;
    }

    Ok(())
}

async fn create_file(path: &PathBuf) -> std::io::Result<File> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::File::create_new(path).await
}
