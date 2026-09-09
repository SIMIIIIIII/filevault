use axum::{
    Router, middleware,
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
        .with_state(state)
}
