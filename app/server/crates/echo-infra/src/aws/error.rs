#[derive(Debug, thiserror::Error)]
pub enum AwsError {
    #[error("Failed uploading part")]
    S3Upload,

    #[error("Failed getting object")]
    S3Get,

    #[error("Failed sending message")]
    SqsSend,

    #[error("Failed receving messages")]
    SqsReceive,

    #[error("Missing object")]
    S3MissingObject,
}
