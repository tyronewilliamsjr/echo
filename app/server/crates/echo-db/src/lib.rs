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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // let result = add(2, 2);
        assert_eq!(4, 4);
    }
}
