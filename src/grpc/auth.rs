use crate::api::auth::{Claims, verify_token};



pub fn authenticate_request<T>(
    request: &tonic::Request<T>,
    secret: &str,
) -> Result<Claims, tonic::Status> {
    let authorization = request
        .metadata()
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| tonic::Status::unauthenticated("token manquant"))?;

    let token = authorization
        .strip_prefix("Bearer ")
        .ok_or_else(|| tonic::Status::unauthenticated("format invalide"))?;

    verify_token(token, secret)
        .map_err(|_| tonic::Status::unauthenticated("token invalide"))
}