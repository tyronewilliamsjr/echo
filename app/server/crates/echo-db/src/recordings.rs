use sqlx::{Executor, Postgres};
use uuid::Uuid;

#[derive(Debug, Copy, Clone, sqlx::Type)]
#[sqlx(type_name = "failure_code", rename_all = "snake_case")]
pub enum FailureCode {
    InvalidState,

    MissingCodec,
    UnsupportedAudioType,

    // Failure can not be retried
    MissingObject,
    ProbeRejected,
    PcmRejected,

    EmptyFile,
    FileTooLarge,
    SizeMismatch,

    NoAudioStream,
    MultiAudioSteams,

    MissingDuration,
    InvalidDuration,
    RecordingTooLong,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "recording_status", rename_all = "snake_case")]
pub enum RecordingStatus {
    Invalid,
    Initial,
    Completed,
    Abort,

    Uploading,
    Uploaded,
    Recovering,

    Validating,
    Validated,
    Processing,
    Processed,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Recording {
    pub client_id: Uuid,
    pub id: Uuid,
    pub user_id: Uuid,
    pub upload_id: Option<String>,
    pub status: RecordingStatus,
    pub failure_code: Option<FailureCode>,
}

pub struct CreateRecording {
    pub client_id: Uuid,
    pub user_id: Uuid,
}

pub struct UpdateRecording {
    pub upload_id: Option<String>,
    pub status: Option<RecordingStatus>,
    pub failure_code: Option<FailureCode>,
}

pub struct ClaimInputs {
    pub failure_code: Option<FailureCode>,
    pub status: Option<RecordingStatus>,
}

pub async fn create<'e, E>(executor: E, input: CreateRecording) -> Result<Recording, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let recording = sqlx::query_as::<_, Recording>(
        "
          INSERT INTO recordings (client_id, user_id)
          values ($1, $2)
          RETURNING *
        ",
    )
    .bind(input.client_id)
    .bind(input.user_id)
    .fetch_one(executor)
    .await?;

    Ok(recording)
}

pub async fn update_by_id<'e, E>(
    executor: E,
    id: Uuid,
    input: UpdateRecording,
) -> Result<Recording, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let mut query = sqlx::QueryBuilder::<Postgres>::new("UPDATE recordings SET ");

    let mut separated = query.separated(", ");

    if let Some(upload_id) = input.upload_id {
        separated.push("upload_id = ");
        separated.push_bind_unseparated(upload_id);
    }

    if let Some(status) = input.status {
        separated.push("status = ");
        separated.push_bind_unseparated(status);
    }

    if let Some(failure_code) = input.failure_code {
        separated.push("failure_code = ");
        separated.push_bind_unseparated(failure_code);
    }

    query.push(" WHERE id = ");
    query.push_bind(id);

    query.push(" RETURNING *");

    query
        .build_query_as::<Recording>()
        .fetch_one(executor)
        .await
}

pub async fn get_by_server_id<'e, E>(executor: E, server_id: Uuid) -> Result<Recording, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let recording = sqlx::query_as::<_, Recording>(
        "
      SELECT *
      FROM recordings
      WHERE id = $1
    ",
    )
    .bind(server_id)
    .fetch_one(executor)
    .await?;

    Ok(recording)
}

pub async fn get_by_client_id<'e, E>(executor: E, client_id: Uuid) -> Result<Recording, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let recording = sqlx::query_as::<_, Recording>(
        "
      SELECT *
      FROM recordings
      WHERE client_id = $1
    ",
    )
    .bind(client_id)
    .fetch_one(executor)
    .await?;

    Ok(recording)
}

pub async fn find_by_client_id<'e, E>(
    executor: E,
    client_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Recording>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let recording = sqlx::query_as::<_, Recording>(
        "
    SELECT *
    FROM recordings
    WHERE client_id = $1 AND user_id = $2
   ",
    )
    .bind(client_id)
    .bind(user_id)
    .fetch_optional(executor)
    .await?;

    Ok(recording)
}

pub async fn find_by_server_id<'e, E>(
    executor: E,
    server_id: Uuid,
) -> Result<Option<Recording>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let recording = sqlx::query_as::<_, Recording>(
        "
    SELECT *
    FROM recordings
    WHERE id = $1
   ",
    )
    .bind(server_id)
    .fetch_optional(executor)
    .await?;

    Ok(recording)
}

pub async fn set_validate<'e, E>(
    executor: E,
    server_id: Uuid,
) -> Result<Option<Recording>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let recording = sqlx::query_as::<_, Recording>(
        "
      UPDATE recordings
      SET status = $2::recording_status, validation_started = NOW()
      WHERE id = $1 AND (status = $3::recording_status OR (status = $3::recording_status AND validation_started < NOW() - INTERVAL '10 minutes') )
      RETURNING *
    ",
    )
    .bind(server_id)
    .bind(RecordingStatus::Validating)
    .bind(RecordingStatus::Uploaded)
    .fetch_optional(executor)
    .await?;

    Ok(recording)
}

pub async fn claim_process<'e, E>(
    executor: E,
    id: Uuid,
    claim_id: Uuid,
) -> Result<Option<Recording>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, Recording>(
        "
      UPDATE recordings
      SET status =
        $2::recording_status,
        processing_claim_id = $4,
        failure_code = NULL,
        processing_claimed = NOW()
      WHERE id = $1
        AND (
          status = $3
          OR (
            status = $2
            AND processing_claimed < NOW() - INTERVAL '30 minutes'
          )
        )
      RETURNING *
    ",
    )
    .bind(id)
    .bind(RecordingStatus::Processing)
    .bind(RecordingStatus::Validated)
    .bind(claim_id)
    .fetch_optional(executor)
    .await
}

async fn finish_processing<'e, E>(
    executor: E,
    id: Uuid,
    claim_id: Uuid,
    status: RecordingStatus,
    failure_code: Option<FailureCode>,
) -> Result<Recording, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, Recording>(
        "
    UPDATE recordings
    SET processing_claimed = NULL, processing_claim_id = NULL, status = $2, failure_code = $3
    WHERE id = $1 AND processing_claim_id = $4 AND status = $5
    RETURNING *
  ",
    )
    .bind(id)
    .bind(status)
    .bind(failure_code)
    .bind(claim_id)
    .bind(RecordingStatus::Processing)
    .fetch_one(executor)
    .await
}

pub async fn complete_processing<'e, E>(
    executor: E,
    id: Uuid,
    claim_id: Uuid,
) -> Result<Recording, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    finish_processing(executor, id, claim_id, RecordingStatus::Processed, None).await
}

pub async fn invalidate_processing<'e, E>(
    executor: E,
    id: Uuid,
    claim_id: Uuid,
    failure_code: FailureCode,
) -> Result<Recording, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    finish_processing(
        executor,
        id,
        claim_id,
        RecordingStatus::Invalid,
        Some(failure_code),
    )
    .await
}

/// Releases the processing claim to allow for a reattempt.
pub async fn release_claim_process<'e, E>(
    executor: E,
    id: Uuid,
    claim_id: Uuid,
) -> Result<Recording, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    finish_processing(executor, id, claim_id, RecordingStatus::Validated, None).await
}

pub async fn set_recovery<'e, E>(
    executor: E,
    client_id: Uuid,
    recovery_id: Uuid,
    user_id: Uuid,
) -> Result<Option<Recording>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let recording = sqlx::query_as::<_, Recording>(
        "
        UPDATE recordings
        SET status = $4, recoverId = $3, recovery_expires = NOW() + INTERVAL '2 minutes'
        WHERE user_id = $1 AND client_id = $2 AND (status in ($4, $5) OR recovery_expires < NOW() )
        RETURNING *
      ",
    )
    .bind(user_id)
    .bind(client_id)
    .bind(recovery_id)
    .bind(RecordingStatus::Recovering)
    .bind(RecordingStatus::Uploading)
    .fetch_optional(executor)
    .await?;

    Ok(recording)
}
