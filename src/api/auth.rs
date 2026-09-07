use axum::{extract::{Request, State}, middleware::Next, response::Response};
use jsonwebtoken::{
    encode,
    decode,
    Header,
    EncodingKey,
    DecodingKey,
    Validation,
    Algorithm
};

use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};
use crate::{api::state::AppState, files_vault_errors::FileVaultError};

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