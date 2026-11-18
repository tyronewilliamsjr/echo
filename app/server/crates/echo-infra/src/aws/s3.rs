use super::error::AwsError;
use aws_sdk_s3::{
    Client,
    operation::{get_object::GetObjectError, head_object::HeadObjectError},
    presigning::PresigningConfig,
    primitives::{ByteStream, DateTime},
    types::{CompletedMultipartUpload, CompletedPart},
};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct UploadUrl {
    part_number: i32,
    url: String,
}

#[derive(Debug, Serialize)]
pub struct UploadedPart {
    part_number: i32,
    size: i64,
    e_tag: String,
}

pub struct ObjectMetadata {
    pub e_tag: String,
    pub content_length: u64,
    pub last_modified: Option<DateTime>,
    pub content_type: Option<String>,
}

const BUCKET: &str = "echo-975661984975-us-east-2-an";

/// Handles  making a request to aws for CreateMultiUpload
/// https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateMultipartUpload.html
pub async fn init_multi_upload(key: &str, content_type: &str) -> Result<String, AwsError> {
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let client = Client::new(&shared_config);

    let response = client
        .create_multipart_upload()
        .bucket(BUCKET)
        .key(key)
        .content_type(content_type)
        .send()
        .await
        .map_err(|_| AwsError::S3Upload)?;

    let upload_id = response.upload_id().ok_or(AwsError::S3Upload)?;
    Ok(upload_id.to_string())
}

pub async fn complete_multi_upload(key: &str, upload_id: &str) -> Result<(), AwsError> {
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let client = Client::new(&shared_config);

    let parts_response = client
        .list_parts()
        .bucket(BUCKET)
        .key(key)
        .send()
        .await
        .map_err(|_| AwsError::S3Upload)?;

    let completed_parts: Vec<CompletedPart> = parts_response
        .parts()
        .iter()
        .map(|part| {
            CompletedPart::builder()
                .part_number(part.part_number().unwrap())
                .e_tag(part.e_tag().unwrap())
                .build()
        })
        .collect();

    let complete_upload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    let _ = client
        .complete_multipart_upload()
        .bucket(BUCKET)
        .key(key)
        .multipart_upload(complete_upload)
        .upload_id(upload_id)
        .send()
        .await
        .map_err(|_| AwsError::S3Upload)?;

    Ok(())
}

/// Handles getting upload urls for multiupload
pub async fn get_upload_url_parts(
    upload_id: &str,
    key: &str,
    parts: Vec<i32>,
) -> Result<Vec<UploadUrl>, AwsError> {
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let client = Client::new(&shared_config);

    let mut signed_urls: Vec<UploadUrl> = Vec::new();

    for part in parts {
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))
            .map_err(|_| AwsError::S3Upload)?;

        let presigned = client
            .upload_part()
            .bucket(BUCKET)
            .key(key)
            .upload_id(upload_id)
            .part_number(part)
            .presigned(presigning_config)
            .await
            .map_err(|_| AwsError::S3Upload)?;

        signed_urls.push(UploadUrl {
            part_number: part,
            url: presigned.uri().to_string(),
        });
    }

    Ok(signed_urls)
}

pub async fn abort_multiupload(upload_id: &str, key: &str) -> Result<(), AwsError> {
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let client = Client::new(&shared_config);

    let _ = client
        .abort_multipart_upload()
        .bucket(BUCKET)
        .key(key)
        .upload_id(upload_id)
        .send()
        .await
        .map_err(|_| AwsError::S3Upload)?;

    Ok(())
}

pub async fn get_list_parts(key: &str, upload_id: &str) -> Result<Vec<UploadedPart>, AwsError> {
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let client = Client::new(&shared_config);

    let result = client
        .list_parts()
        .bucket(BUCKET)
        .key(key)
        .upload_id(upload_id)
        .send()
        .await
        .map_err(|_| AwsError::S3Upload)?;

    let mut parts: Vec<UploadedPart> = vec![];

    for part in result.parts() {
        if let (Some(part_number), Some(size), Some(e_tag)) =
            (part.part_number(), part.size(), part.e_tag())
        {
            parts.push(UploadedPart {
                part_number,
                size,
                e_tag: e_tag.to_owned(),
            })
        }
    }

    Ok(parts)
}

pub async fn get_file_info(client: &Client, key: &str) -> Result<ObjectMetadata, AwsError> {
    let response = client
        .head_object()
        .bucket(BUCKET)
        .key(key)
        .send()
        .await
        .map_err(|err| match err.into_service_error() {
            HeadObjectError::NotFound(_) => AwsError::S3MissingObject,
            _ => AwsError::S3Get,
        })?;

    let content_length = u64::try_from(response.content_length().ok_or_else(|| AwsError::S3Get)?)
        .map_err(|_| AwsError::S3Get)?;

    let metadata = ObjectMetadata {
        e_tag: response.e_tag().ok_or_else(|| AwsError::S3Get)?.to_owned(),
        content_length: content_length,
        last_modified: response.last_modified,
        content_type: response.content_type.clone(),
    };

    Ok(metadata)
}

pub async fn get_file_by_key(client: &Client, key: &str) -> Result<ByteStream, AwsError> {
    let response = client
        .get_object()
        .bucket(BUCKET)
        .key(key)
        .send()
        .await
        .map_err(|err| match err.into_service_error() {
            GetObjectError::NoSuchKey(_) => AwsError::S3MissingObject,
            _ => AwsError::S3Get,
        })?;

    Ok(response.body)
}

pub async fn delete_file(client: &Client, key: &str) -> Result<(), AwsError> {
    client
        .delete_object()
        .bucket(BUCKET)
        .key(key)
        .send()
        .await
        .map_err(|_| AwsError::S3MissingObject)?;

    Ok(())
}
