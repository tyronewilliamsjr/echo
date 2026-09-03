use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres, postgres::PgPool};
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateAccount {
    pub user_id: Uuid,
    pub provider: String,
    pub provider_id: String,
}

async fn create<'e, E>(executor: E, input: CreateAccount) -> Result<Account, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let account = sqlx::query_as::<_, Account>(
        "
            INSERT INTO accounts (user_id, provider, provider_id)
            VALUES ($1, $2, $3)
            RETURNING *
        ",
    )
    .bind(input.user_id)
    .bind(input.provider)
    .bind(input.provider_id)
    .fetch_one(executor)
    .await?;

    Ok(account)
}

async fn find_by_user<'e, E>(executor: E, user_id: Uuid) -> Result<Option<Account>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let account = sqlx::query_as::<_, Account>(
        "
            SELECT *
            FROM accounts
            WHERE user_id = $1
        ",
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await?;

    Ok(account)
}
