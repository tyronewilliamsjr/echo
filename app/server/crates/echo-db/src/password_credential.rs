use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PasswordCredential {
    pub user_id: Uuid,
    pub hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create password credential record for a user. Stores hash directly, be sure to handle hashing before calling.
pub async fn create<'e, E>(executor: E, user_id: Uuid, hash: String) -> Result<(), sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(
        "
            INSERT INTO password_credentials (user_id, hash)
            VALUES ($1, $2)
            RETURNING *
        ",
    )
    .bind(user_id)
    .bind(hash)
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn find_by_user<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<Option<PasswordCredential>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let creds = sqlx::query_as::<_, PasswordCredential>(
        "
      SELECT *
      FROM password_credentials
      WHERE user_id = $1
    ",
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await?;

    Ok(creds)
}

pub async fn update<'e, E>(
    executor: E,
    user_id: Uuid,
    hash: String,
) -> Result<PasswordCredential, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let cred = sqlx::query_as::<_, PasswordCredential>(
        "
            UPDATE password_credentials
            SET hash = $2
            WHERE user_id = $1
            RETURNING *
        ",
    )
    .bind(user_id)
    .bind(hash)
    .fetch_one(executor)
    .await?;

    Ok(cred)
}
