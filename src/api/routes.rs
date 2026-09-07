use axum::{
    Router, 
    middleware,
    routing::{get, post},
};

use crate::api::{
    auth::middleware_auth, handlers::{
        delete_file, download_file, list_files, login, register, statistics, upload_file
    }, state::AppState
};

pub fn build_router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/files", get(list_files).post(upload_file))
        .route("/files/:id", get(download_file).delete(delete_file))
        .route("/stats", get(statistics))
        .route_layer(middleware::from_fn_with_state(state.clone(), middleware_auth));

    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/health", get(|| async { "ok" }))
        .merge(protected_routes)
        .with_state(state)
}