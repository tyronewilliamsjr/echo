use sqlx::{Executor, Postgres};
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Recording {
    pub client_id: Uuid,
    pub id: Uuid,
    pub user_id: Uuid,
    pub upload_id: Option<String>,
    pub status: String,
}

pub struct CreateRecording {
    pub client_id: Uuid,
    pub user_id: Uuid,
}

pub struct UpdateRecording {
    pub upload_id: Option<String>,
    pub status: Option<String>,
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
        SET status = 'recovering', recoverId = $3, recovery_expires = NOW() + INTERVAL '1 minute'
        WHERE user_id = $1 AND client_id = $2 AND (status in ('recovering', 'uploading') OR recovery_expires < NOW() )
        RETURNING *
      ",
    )
    .bind(user_id)
    .bind(client_id)
    .bind(recovery_id)
    .fetch_optional(executor)
    .await?;

    Ok(recording)
}
