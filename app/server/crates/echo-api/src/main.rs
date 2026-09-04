use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
mod app;
mod app_state;
mod auth;
mod error;
mod middleware;
mod users;

use app_state::AppState;
use error::ApiError;

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "echo_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_url = "NO LEAKS";

    init_tracing();
    let pool = echo_db::connect(db_url).await?;
    let app_state = AppState {
        pool,
        cookie_secure: std::env::var("SECURE_COOKIES")
            .map(|v| v == "true")
            .unwrap_or(!cfg!(debug_assertions)),
    };

    let application = app::create_app(app_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    tracing::info!("API server starting");
    axum::serve(listener, application).await?;

    Ok(())
}
