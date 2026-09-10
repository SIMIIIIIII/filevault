use axum::{
    Router,
    body::Body,
    extract::DefaultBodyLimit,
    http::{Request, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, post},
};

use crate::api::{
    auth::{middleware_auth, mtls_middleware},
    handlers::{
        delete_file, download_file, health_check, list_files, login, register, statistics,
        upload_file,
    },
    state::AppState,
};
use std::{sync::OnceLock, time::Duration};

static RATE_LIMITER: OnceLock<std::sync::Arc<crate::api::state::RateLimiter>> = OnceLock::new();

async fn rate_limit_middleware(
    request: Request<Body>,
    next: middleware::Next,
) -> axum::response::Response {
    let limiter = RATE_LIMITER.get_or_init(crate::api::state::RateLimiter::new);
    let limited = {
        let mut requests = limiter.requests.lock().unwrap();
        if requests.0.elapsed() >= Duration::from_secs(60) {
            *requests = (std::time::Instant::now(), 0);
        }
        if requests.1 >= 100 {
            true
        } else {
            requests.1 += 1;
            false
        }
    };
    if limited {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    next.run(request).await
}

pub fn build_router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/files", get(list_files).post(upload_file))
        .route("/files/:id", get(download_file).delete(delete_file))
        .route("/stats", get(statistics))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            middleware_auth,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            mtls_middleware,
        ));

    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/health", get(health_check))
        .merge(protected_routes)
        .route_layer(middleware::from_fn(rate_limit_middleware))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        .with_state(state)
}
