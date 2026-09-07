use axum::{extract::{Request, State}, middleware::Next, response::Response};
use axum_server::accept::Accept;
use axum_server::tls_rustls::{RustlsAcceptor, RustlsConfig};
use jsonwebtoken::{
    encode,
    decode,
    Header,
    EncodingKey,
    DecodingKey,
    Validation,
    Algorithm
};
use rustls::{RootCertStore, ServerConfig};
use rustls::server::WebPkiClientVerifier;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tower_http::add_extension::AddExtension;

use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};
use crate::{api::state::AppState, files_vault_errors::FileVaultError};

#[derive(Clone)]
pub struct ClientIdentity(pub String);

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64, // subject : user_id
    pub iat: usize, // issued at
    pub exp: usize, // expiration
}

pub fn generate_token(user_id: i64, secret: &str) -> Result<String, FileVaultError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id,
        iat: now.timestamp() as usize,
        exp: (now + Duration::hours(24)).timestamp() as usize, // token valide 24h
    };

    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
    .map_err(|e| FileVaultError::ConnectionError(format!("génération token: {e}")))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, FileVaultError> {
    let validation = Validation::new(Algorithm::HS256);
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation)
    .map(|data| data.claims)
    .map_err(|e| FileVaultError::ConnectionError(format!("token invalide: {e}")))
}

pub async fn middleware_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, FileVaultError> {
    let token = req.headers()
    .get(axum::http::header::AUTHORIZATION)
    .and_then(|v| v.to_str().ok())
    .and_then(|v| v.strip_prefix("Bearer "))
    .ok_or(FileVaultError::ConnectionError("token manquant".into()))?;

    let claims = verify_token(token, &state.jwt_secret)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

pub async fn build_mtls_config() -> anyhow::Result<RustlsConfig> {
    // Load de CA to verify client certificat
    let ca_cert = tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/pki/ca/ca.crt")).await?;
    let mut root_store = RootCertStore::empty();
    
    let ca_certs = rustls_pemfile::certs(&mut &ca_cert[..])
        .collect::<Result<Vec<_>, _>>()?;

    for cert in ca_certs {
        root_store.add(cert)?;
    }

    // Verify if the client cetrificate was signed by my CA
    let client_verify = WebPkiClientVerifier::builder(
        Arc::new(root_store),
    ).build()?;

    // Load server certificat and key
    let server_cert = tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/pki/server/server.crt")).await?;
    let server_key = tokio::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/pki/server/server.key")).await?;

    let certs = rustls_pemfile::certs(&mut &server_cert[..])
        .collect::<Result<Vec<_>, _>>()?;

    let key = rustls_pemfile::private_key(&mut &server_key[..])?
        .ok_or_else(|| anyhow::anyhow!("Pas de clé privée trouvée"))?;

    let config = ServerConfig::builder()
        .with_client_cert_verifier(client_verify)
        .with_single_cert(certs, key)?;

    Ok(RustlsConfig::from_config(Arc::new(config)))
}


pub async fn mtls_middleware(
    req: Request,
    next: Next,
) -> Response {
    // Only the trusted `Accept` layer can set this extension (derived from the verified
    // client cert), so falling back here just means the connection didn't go through TLS
    // (e.g. tests calling the router directly), not that identity was spoofed.
    let identity = req.extensions()
        .get::<ClientIdentity>()
        .map(|i| i.0.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    tracing::info!(client = %identity, "Requête mTLS authentifiée");
    next.run(req).await
}

#[derive(Clone)]
pub struct MtlsAcceptor {
    inner: RustlsAcceptor,
}

impl MtlsAcceptor {
    pub fn new(config: RustlsConfig) -> Self {
        Self { inner: RustlsAcceptor::new(config) }
    }
}

impl<S> Accept<TcpStream, S> for MtlsAcceptor
where
    S: Send + 'static,
{
    type Stream = TlsStream<TcpStream>;
    type Service = AddExtension<S, ClientIdentity>;
    type Future = Pin<Box<dyn Future<Output = io::Result<(Self::Stream, Self::Service)>> + Send>>;

    fn accept(&self, stream: TcpStream, service: S) -> Self::Future {
        let inner = self.inner.clone();
        Box::pin(async move {
            let (tls_stream, service) = inner.accept(stream, service).await?;
            let identity = extract_client_cn(&tls_stream).unwrap_or_else(|| "anonymous".to_string());
            let service = AddExtension::new(service, ClientIdentity(identity));
            Ok((tls_stream, service))
        })
    }
}

fn extract_client_cn(tls_stream: &TlsStream<TcpStream>) -> Option<String> {
    let (_, server_conn) = tls_stream.get_ref();
    let cert = server_conn.peer_certificates()?.first()?;
    let (_, x509) = x509_parser::parse_x509_certificate(cert.as_ref()).ok()?;
    x509.subject()
        .iter_common_name()
        .next()?
        .as_str()
        .ok()
        .map(|s| s.to_string())
}