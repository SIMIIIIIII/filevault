use sqlx::{Pool, Postgres};
use tokio::sync::mpsc;
use tonic::transport::{Identity, ServerTlsConfig};

use crate::{
    cache::redis_cache::RedisCache,
    cli_helpers::RuntimeMode,
    grpc::server::{FileVaultService, filevault::file_vault_server::FileVaultServer},
    kafka::{consumer::KafkaConsumer, handlers::Transaction, producer::KafkaProducer},
};

pub async fn run(
    pool: Pool<Postgres>,
    host: String,
    port: u64,
    jwt_secret: String,
    runtime_mode: RuntimeMode,
) -> Result<(), String> {
    let addr = format!("[{}]:{}", host, port)
        .parse::<std::net::SocketAddr>()
        .map_err(|e| e.to_string())?;

    let kafka_producer =
        KafkaProducer::new("localhost:9094", "file-events").map_err(|e| e.to_string())?;

    let cache = match RedisCache::new("redis://127.0.0.1:6379/", 3600).await {
        Ok(cache) => Some(cache),
        Err(error) => {
            tracing::warn!(%error, "Redis unavailable; cache disabled");
            None
        }
    };

    let service = FileVaultService {
        pool,
        jwt_secret,
        max_upload_bytes: 100 * 1024 * 1024,
        transaction: Transaction::new(kafka_producer).map_err(|e| e.to_string())?,
        cache: cache.clone(),
    };

    let cert = tokio::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/pki/server/server.crt"
    ))
    .await
    .map_err(|e| e.to_string())?;

    let key = tokio::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/pki/server/server.key"
    ))
    .await
    .map_err(|e| e.to_string())?;

    let identity = Identity::from_pem(cert, key);
    let tls_config = ServerTlsConfig::new().identity(identity);

    let mut server = configure_tls(tls_config)?;

    // Créer un channel pour le signal Ctrl+C
    let (tx_shutdown, rx_shutdown) = mpsc::channel::<()>(1);

    // Spawner une tâche pour écouter Ctrl+C
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("[SHUTDOWN]: Ctrl+C reçu, arrêt gracieux...");
        let _ = tx_shutdown.send(()).await;
    });

    // Spawner le consumer Kafka
    tokio::spawn({
        let mut consumer = KafkaConsumer::new(
            "localhost:9094",
            "filevault-group",
            "file-events",
            runtime_mode,
            cache,
        )
        .map_err(|e| e.to_string())?;

        async move {
            tracing::info!("[KAFKA]: Consumer démarré");
            if let Err(e) = consumer.start().await {
                tracing::error!(err = %e, "[KAFKA]: Consumer start error");
            }
            tracing::info!("[KAFKA]: Consumer arrêté");
        }
    });

    println!("[SERVER]: starting grpc on {addr}");

    // Créer une future pour le serveur avec graceful shutdown
    let server_future = server
        .add_service(FileVaultServer::new(service))
        .serve_with_shutdown(addr, async {
            let mut rx = rx_shutdown;
            let _ = rx.recv().await;
            tracing::info!("[SHUTDOWN]: Server gRPC arrêt demandé");
        });

    // Attendre le serveur (avec signal Ctrl+C)
    server_future.await.map_err(|e| e.to_string())?;

    tracing::info!("[SHUTDOWN]: Server arrêté, fin du programme");
    Ok(())
}

fn configure_tls(tls_config: ServerTlsConfig) -> Result<tonic::transport::Server, String> {
    if rustls::crypto::CryptoProvider::get_default().is_none()
        && rustls::crypto::ring::default_provider()
            .install_default()
            .is_err()
        && rustls::crypto::CryptoProvider::get_default().is_none()
    {
        return Err("Failed to configure the Rustls crypto provider".to_string());
    }

    tonic::transport::Server::builder()
        .tls_config(tls_config)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_identity_returns_error_instead_of_crypto_provider_panic() {
        let tls_config = ServerTlsConfig::new().identity(Identity::from_pem("", ""));
        assert!(configure_tls(tls_config).is_err());
    }
}
