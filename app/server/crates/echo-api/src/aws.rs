use aws_sdk_s3::{
    Client,
    presigning::PresigningConfig,
    types::{CompletedMultipartUpload, CompletedPart},
};
use serde::Serialize;
use std::time::Duration;

use crate::error::ApiError;

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

const BUCKET: &str = "NOLEAKS";

/// Handles  making a request to aws for CreateMultiUpload
/// https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateMultipartUpload.html
pub async fn init_multi_upload(key: &str, content_type: &str) -> Result<String, ApiError> {
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
        .map_err(|err| ApiError::AwsMultipartUpload(Box::new(err)))?;

    let upload_id = response
        .upload_id()
        .ok_or(ApiError::AwsError("failed to create uploadid".to_owned()))?;
    Ok(upload_id.to_string())
}

pub async fn complete_multi_upload(key: &str, upload_id: &str) -> Result<(), ApiError> {
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
        .map_err(|err| ApiError::AwsListParts(Box::new(err)))?;

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
        .map_err(|err| ApiError::AwsCompleteUpload(Box::new(err)))?;

    Ok(())
}

/// Handles getting upload urls for multiupload
pub async fn get_upload_url_parts(
    upload_id: &str,
    key: &str,
    parts: Vec<i32>,
) -> Result<Vec<UploadUrl>, ApiError> {
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let client = Client::new(&shared_config);

    let mut signed_urls: Vec<UploadUrl> = Vec::new();

    for part in parts {
        let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))?;

        let presigned = client
            .upload_part()
            .bucket(BUCKET)
            .key(key)
            .upload_id(upload_id)
            .part_number(part)
            .presigned(presigning_config)
            .await
            .map_err(|err| ApiError::AwsPresignedUpload(Box::new(err)))?;

        signed_urls.push(UploadUrl {
            part_number: part,
            url: presigned.uri().to_string(),
        });
    }

    Ok(signed_urls)
}

pub async fn abort_multiupload(upload_id: &str, key: &str) -> Result<(), ApiError> {
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
        .map_err(|err| ApiError::AwsAbortMultiUpload(Box::new(err)))?;

    Ok(())
}

pub async fn get_list_parts(key: &str, upload_id: &str) -> Result<Vec<UploadedPart>, ApiError> {
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
        .map_err(|err| ApiError::AwsListParts(Box::new(err)))?;

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
