use std::sync::{Arc, Mutex};
use std::time::Instant;

use sqlx::PgPool;

pub struct RateLimiter {
    pub requests: Mutex<(Instant, u32)>,
}

impl RateLimiter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            requests: Mutex::new((Instant::now(), 0)),
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: String,
    pub max_upload_bytes: u64,
}
