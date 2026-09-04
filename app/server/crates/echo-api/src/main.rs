mod app;
mod app_state;
mod auth;
mod error;
mod middleware;
mod users;

use app_state::AppState;
use error::ApiError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = "NO LEAKS";

    let pool = echo_db::connect(db_url).await?;
    let app_state = AppState {
        pool,
        cookie_secure: std::env::var("SECURE_COOKIES")
            .map(|v| v == "true")
            .unwrap_or(!cfg!(debug_assertions)),
    };

    let application = app::create_app(app_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, application).await?;

    Ok(())
}
