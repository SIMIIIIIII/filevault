use sqlx::{Pool, Postgres};
use tonic::transport::{Identity, ServerTlsConfig};

use crate::grpc::server::{filevault::file_vault_server::FileVaultServer, FileVaultService};

pub async fn run(
    pool: Pool<Postgres>,
    host: String,
    port: u64,
    jwt_secret: String,
) -> Result<(), String> {
    let addr = format!("[{}]:{}", host, port)
        .parse::<std::net::SocketAddr>()
        .map_err(|e| e.to_string())?;

    let service = FileVaultService {
        pool,
        jwt_secret,
        max_upload_bytes: 100 * 1024 * 1024,
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
    println!("[SERVER]: starting grpc on {addr}");
    server
        .add_service(FileVaultServer::new(service))
        .serve(addr)
        .await
        .map_err(|e| e.to_string())?;

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
