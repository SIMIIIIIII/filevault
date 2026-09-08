use sqlx::{Pool, Postgres};
use tonic::transport::{Identity, ServerTlsConfig};

use crate::grpc::server::{FileVaultService, filevault::file_vault_server::FileVaultServer};

pub async fn run(pool: Pool<Postgres>, host: String, port: u64, jwt_secret: String) -> Result<(), String> {
    let addr = format!("[{}]:{}", host, port)
                .parse::<std::net::SocketAddr>()
                .map_err(|e| e.to_string())?;

            let service = FileVaultService {
                pool,
                jwt_secret,
                max_upload_bytes: 100 * 1024 * 1024
            };

            let cert = tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/pki/server/server.crt"))
                .await
                .map_err(|e| e.to_string())?;

            let key = tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/pki/server/server.key"))
                .await
                .map_err(|e| e.to_string())?;
            let identity = Identity::from_pem(cert, key);
            let tls_config = ServerTlsConfig::new().identity(identity);

            println!("[SERVER] : grcp stated on {addr} ");
            tonic::transport::Server::builder()
                .tls_config(tls_config)
                .map_err(|e| e.to_string())?
                .add_service(FileVaultServer::new(service))
                .serve(addr)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
}