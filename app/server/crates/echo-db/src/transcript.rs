use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Transcript {
    pub id: Uuid,
    pub recording_id: Uuid,
    pub user_id: Uuid,

    pub content: String,
    pub created_at: DateTime<Utc>,
}

pub struct CreateTranscript {
    pub recording_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
}

pub async fn create<'e, E>(executor: E, input: CreateTranscript) -> Result<Transcript, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, Transcript>(
        "
          INSERT INTO transcripts (recording_id, content, user_id)
          values ($1, $2, $3)
          RETURNING *
        ",
    )
    .bind(input.recording_id)
    .bind(input.content)
    .bind(input.user_id)
    .fetch_one(executor)
    .await
}

pub async fn get_by_id<'e, E>(executor: E, id: Uuid) -> Result<Transcript, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, Transcript>(
        "
        SELECT *
        FROM transcripts
        WHERE id = $1
        RETURNING *
      ",
    )
    .bind(id)
    .fetch_one(executor)
    .await
}

pub async fn get_by_recording_id<'e, E>(
    executor: E,
    recording_id: Uuid,
) -> Result<Transcript, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_as::<_, Transcript>(
        "
          SELECT *
          FROM transcripts
          WHERE recording_id = $1
        ",
    )
    .bind(recording_id)
    .fetch_one(executor)
    .await
}
