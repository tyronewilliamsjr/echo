use std::path::PathBuf;

use tokio::io::AsyncRead;
use uuid::Uuid;

use aws_sdk_s3::Client;

use crate::aws::{error::AwsError, s3};

fn generate_key(id: Uuid) -> String {
    format!("recording/{id}")
}

pub type RecordingReader = Box<dyn AsyncRead + Unpin + Send>;

pub struct RecordingMetadata {
    pub content_length: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Missing Object")]
    MissingObject,

    #[error("Fail")]
    Aws(#[from] AwsError),

    #[error("Fail")]
    File(#[from] std::io::Error),
}

pub trait RecordingStore {
    async fn get_metadata(&self, id: Uuid) -> Result<RecordingMetadata, StorageError>;
    async fn get_recording(&self, id: Uuid) -> Result<RecordingReader, StorageError>;
}

pub struct LocalRecordingStorage {
    pub root: PathBuf,
}

impl LocalRecordingStorage {
    pub fn new(path: &str) -> Self {
        let root = PathBuf::from(path);
        LocalRecordingStorage { root }
    }

    pub async fn copy_local_to_root(
        self: &Self,
        id: Uuid,
        source: &str,
    ) -> Result<u64, std::io::Error> {
        let mut destination = self.root.clone();
        destination.push(id.to_string());

        tokio::fs::copy(source, destination).await
    }
}

impl RecordingStore for LocalRecordingStorage {
    async fn get_metadata(&self, id: Uuid) -> Result<RecordingMetadata, StorageError> {
        let path = self.root.join(id.to_string());
        println!("{:?}", path);
        let file = tokio::fs::metadata(path).await?;

        Ok(RecordingMetadata {
            content_length: file.len(),
        })
    }

    async fn get_recording(&self, id: Uuid) -> Result<RecordingReader, StorageError> {
        let path = self.root.join(id.to_string());
        println!("{:?}", path);
        let file = tokio::fs::File::open(path).await?;

        Ok(Box::new(file))
    }
}

pub struct RecordingStorage {
    s3: Client,
}

impl RecordingStorage {
    pub fn new(client: Client) -> Self {
        Self { s3: client }
    }
}

impl RecordingStore for RecordingStorage {
    async fn get_metadata(self: &Self, id: Uuid) -> Result<RecordingMetadata, StorageError> {
        let key = generate_key(id);
        let metadata = s3::get_file_info(&self.s3, &key)
            .await
            .map_err(|err| match err {
                AwsError::S3MissingObject => StorageError::MissingObject,
                err => StorageError::Aws(err),
            })?;

        Ok(RecordingMetadata {
            content_length: metadata.content_length,
        })
    }

    async fn get_recording(self: &Self, id: Uuid) -> Result<RecordingReader, StorageError> {
        let key = generate_key(id);
        let stream = s3::get_file_by_key(&self.s3, &key)
            .await
            .map_err(|err| match err {
                AwsError::S3MissingObject => StorageError::MissingObject,
                err => StorageError::Aws(err),
            })?;

        Ok(Box::new(stream.into_async_read()))
    }
}
