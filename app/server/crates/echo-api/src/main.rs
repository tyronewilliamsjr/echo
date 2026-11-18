use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
mod app;
mod app_state;
mod auth;
mod error;
mod middleware;
mod recordings;
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
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    init_tracing();
    let pool = echo_db::connect(&database_url).await?;
    let app_state = AppState {
        pool,
        cookie_secure: std::env::var("SECURE_COOKIES")
            .map(|v| v == "true")
            .unwrap_or(!cfg!(debug_assertions)),
    };

    let application = app::create_app(app_state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    let addr = listener.local_addr()?;

    tracing::info!(%addr, "API server listening");
    axum::serve(listener, application).await?;

    Ok(())
}
