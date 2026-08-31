use sqlx::postgres::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};


#[derive(Clone, Debug,sqlx::FromRow)]
pub struct User {
    pub id: Uuid,

    pub created_date: DateTime<Utc>,
    pub updated_date: DateTime<Utc>
}


pub async fn find_user_by_id(pool: &PgPool, user_id: &str) -> Result<i32, sqlx::Error> {
    sqlx::query("SELECT * from users = ?")
        .bind(user_id)
        .execute(pool)
        .await?;

    Ok(1)
}


pub fn add_user(pool: &PgPool) {

}