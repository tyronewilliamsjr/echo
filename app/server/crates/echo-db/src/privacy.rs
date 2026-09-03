use chrono::{DateTime, Utc};
use sqlx::Executor;
use sqlx::postgres::{PgPool, PgTransaction, Postgres};
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Privacy {
    pub id: Uuid,
    pub user_id: Uuid,
    pub version: i32,
    pub is_current: bool,
    pub retain_audio: bool,
    pub retain_transcript: bool,
    pub allow_embedding: bool,
    pub allow_reminder: bool,
    pub created_at: DateTime<Utc>,
}

pub struct CreatePrivacy {
    pub user_id: Uuid,
    pub version: i32,
    pub is_current: bool,
    pub retain_audio: bool,
    pub retain_transcript: bool,
    pub allow_embedding: bool,
    pub allow_reminder: bool,
}

pub async fn find_current<'e, E>(executor: E, user_id: Uuid) -> Result<Option<Privacy>, sqlx::Error>
where
    E: Executor<'e, Database = Postgres>,
{
    let policy = sqlx::query_as::<_, Privacy>(
        "
            SELECT *
            FROM user_privacy
            WHERE user_id = $1 AND is_current=true
        ",
    )
    .bind(user_id)
    .fetch_optional(executor)
    .await?;

    Ok(policy)
}

pub async fn create_default(tx: &mut PgTransaction<'_>, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
		"
			INSERT INTO user_privacy (user_id, version, is_current, retain_audio, retain_transcript, allow_embedding, allow_reminder)
			VALUES ($1, 1, TRUE, TRUE,TRUE,TRUE,TRUE)
		"
	)
	.bind(user_id)
	.execute(&mut **tx)
	.await?;

    Ok(())
}

// pub async fn create(
//     tx: &mut PgTransaction<'_>,
//     input: Create_Privacy,
// ) -> Result<Privacy, sqlx::Error> {
//     sqlx::query_as::<_, Privacy>().fetch_one(exec).await?
// }
