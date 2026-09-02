use crate::privacy;
use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres, postgres::PgPool};
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateUser {
    pub email: String,
}

async fn create<'e, E>(executor: E, input: CreateUser) -> Result<User, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let user = sqlx::query_as::<_, User>(
        "
            INSERT INTO users (email)
            VALUES ($1)
            RETURNING *
        ",
    )
    .bind(input.email)
    .fetch_one(executor)
    .await?;

    Ok(user)
}

/// Creates user and their default settings for versioning.
pub async fn create_user(pool: &PgPool, input: CreateUser) -> Result<User, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let user = create(&mut *tx, input).await?;

    privacy::create_default(&mut tx, user.id).await?;

    tx.commit().await?;
    Ok(user)
}

pub async fn find_by_id(pool: &PgPool, user_id: Uuid) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "
            SELECT *
            FROM users
            WHERE id=$1
        ",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub async fn find_by_email(pool: &PgPool, email: String) -> Result<Option<User>, sqlx::Error> {
    let user = sqlx::query_as::<_, User>(
        "
        SELECT *
        FROM users
        WHERE email=$1
     ",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

pub fn add_user(pool: &PgPool) {}
