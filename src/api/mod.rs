pub mod state;
pub mod error;
pub mod auth;
pub mod handlers;
pub mod routes;

use sqlx::PgPool;
use state::AppState;
use tower_http::trace::TraceLayer;

pub async fn run(host: String, port: u64, pool: PgPool, jwt_secret: String) -> std::io::Result<()> {
    let state = AppState { pool, jwt_secret, max_upload_bytes: 100 * 1024 * 1024 };

    let app = routes::build_router(state)
        .layer(TraceLayer::new_for_http());
    
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    println!("[API]: listening on {host}:{port}");
    axum::serve(listener, app).await
}