use crate::api::auth::{verify_token, Claims};

pub fn authenticate_request<T>(
    request: &tonic::Request<T>,
    secret: &str,
) -> Result<Claims, Box<tonic::Status>> {
    let authorization = request
        .metadata()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| Box::new(tonic::Status::unauthenticated("token manquant")))?;

    let token = authorization
        .strip_prefix("Bearer ")
        .ok_or_else(|| Box::new(tonic::Status::unauthenticated("format invalide")))?;

    verify_token(token, secret)
        .map_err(|_| Box::new(tonic::Status::unauthenticated("token invalide")))
}
