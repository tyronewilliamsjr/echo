mod error;
mod validate;

use echo_db;
use echo_infra::aws::sqs::Queue;
use echo_infra::recording_storage::RecordingStorage;
use echo_worker::{ProcessingState, ValidationState};
use echo_worker::{
    WorkerState, error::WorkerError, transcribe::listen_transcribe, validate::listen_validation,
};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use whisper_rs::{WhisperContext, WhisperContextParameters};

use std::path::PathBuf;
use std::sync::Arc;

async fn verify_model() -> Result<PathBuf, WorkerError> {
    let path_buf = match std::env::var("WHISPER_MODEL_PATH") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("models")
            .join("ggml-base.en.bin"),
    };

    match tokio::fs::metadata(&path_buf).await {
        Ok(data) if data.is_file() => Ok(path_buf),
        Ok(_) => {
            tracing::error!("model not found, {}", path_buf.display());
            Err(WorkerError::ModelMissing)
        }
        Err(err) => return Err(WorkerError::IO(err)),
    }
}

#[tokio::main]
async fn main() -> Result<(), WorkerError> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "echo_worker=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let queue_validation_url =
        std::env::var("QUEUE_VALIDATION_URL").expect("QUEUE_VALIDATION_URL must be set");
    let queue_process_url =
        std::env::var("QUEUE_PROCESS_URL").expect("QUEUE_PROCESS_URL must be set");

    let pool = echo_db::connect(&database_url).await?;
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&shared_config);
    let sqs_client = aws_sdk_sqs::Client::new(&shared_config);
    let queues = Queue::new(sqs_client, queue_validation_url, queue_process_url);
    let storage = RecordingStorage::new(s3_client);

    let model_path = verify_model().await?;

    let context = tokio::task::spawn_blocking(move || {
        WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
            .map_err(|err| WorkerError::Whisper(err))
    })
    .await
    .map_err(|err| WorkerError::TaskJoin(err))??;

    let processing_state = ProcessingState {
        pool,
        storage: Arc::new(storage),
        context: Arc::new(context),
    };

    // listen_validation(&state).await?;
    listen_transcribe(&processing_state, &queues).await?;
    Ok(())
}
