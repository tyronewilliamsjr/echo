use chrono::{DateTime, Utc};
use sqlx::Executor;
use sqlx::postgres::{PgPool, PgTransaction, Postgres};
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Session {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub struct SessionCreate {
    pub user_id: Uuid,
    pub token_hash: Vec<u8>,
    pub expires_at: DateTime<Utc>,
}

pub async fn create<'e, E>(executor: E, input: SessionCreate) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        "
            INSERT INTO sessions (user_id, token_hash, expires_at)
            VALUES ($1, $2, $3)
            RETURNING *
        ",
    )
    .bind(input.user_id)
    .bind(input.token_hash)
    .bind(input.expires_at)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn find_by_hash<'e, E>(executor: E, hash: Vec<u8>) -> Result<Option<Session>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let session = sqlx::query_as::<_, Session>(
        "
            SELECT *
            FROM sessions
            WHERE token_hash=$1
        ",
    )
    .bind(hash)
    .fetch_optional(executor)
    .await?;

    Ok(session)
}
