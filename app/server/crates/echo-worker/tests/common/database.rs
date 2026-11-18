use echo_db::{
    recordings::Recording,
    users::{CreateUser, User},
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_user(pool: &PgPool) -> Result<User, sqlx::error::Error> {
    let email = format!("{}@email.com", Uuid::new_v4());
    echo_db::users::create(pool, CreateUser { email }).await
}

pub async fn create_recording(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Recording, sqlx::error::Error> {
    echo_db::recordings::create(
        pool,
        echo_db::recordings::CreateRecording {
            client_id: Uuid::new_v4(),
            user_id,
        },
    )
    .await
}
