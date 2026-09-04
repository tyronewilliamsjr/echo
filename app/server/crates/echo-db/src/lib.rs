use sqlx::postgres::{PgPool, PgPoolOptions};

pub mod account;
pub mod password_credential;
pub mod privacy;
pub mod sessions;
pub mod users;

pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}
