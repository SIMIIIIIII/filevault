pub mod state;
pub mod error;
pub mod auth;
pub mod handlers;
pub mod routes;

use sqlx::PgPool;
use state::AppState;
use tower_http::trace::TraceLayer;
use auth::MtlsAcceptor;
use std::net::SocketAddr;

pub async fn run(host: String, port: u64, pool: PgPool, jwt_secret: String) -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    // Charger la config TLS avec vérification du certificat client (mTLS)
    let tls_config = auth::build_mtls_config().await?;

    let state = AppState { pool, jwt_secret, max_upload_bytes: 100 * 1024 * 1024 };

    let app = routes::build_router(state)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}").parse()?
    } else {
        format!("{host}:{port}").parse()?
    };

    tracing::info!("FileVault HTTPS started on https://{addr}");

    axum_server::bind(addr)
        .acceptor(MtlsAcceptor::new(tls_config))
        .serve(app.into_make_service())
        .await?;
    Ok(())
    
    /* 
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    println!("[API]: listening on {host}:{port}");
    axum::serve(listener, app).await
    */
}