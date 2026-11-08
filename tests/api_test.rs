#[cfg(test)]
mod tests {
    use std::{
        env,
        time::{SystemTime, UNIX_EPOCH},
    };

    use file_vault::{
        api::{routes, state::AppState},
        db::{self},
    };

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use tower::util::ServiceExt;
    use tower_http::trace::TraceLayer;

    async fn construct_app() -> Result<Router, String> {
        let database_url = env::var("DATABASE_URL_TEST")
            .map_err(|_| "DATABASE_URL_TEST wasn't defined".to_string())?;
        let pool = db::connexion_db(Some(database_url))
            .await
            .map_err(|error| format!("database connection failed: {error}"))?;

        let jwt_secret = env::var("JWT_SECRET_TEST")
            .map_err(|_| "JWT_SECRET_TEST wasn't defined".to_string())?;

        let state = AppState {
            pool,
            jwt_secret,
            max_upload_bytes: 100 * 1024 * 1024,
        };

        let app = routes::build_router(state).layer(TraceLayer::new_for_http());

        Ok(app)
    }

    fn get_body_login(email: Option<String>) -> String {
        let e = email.unwrap_or("inconnu@example.com".to_string());
        serde_json::json!({
            "email": e,
            "password": "mauvais-mot-de-passe"
        })
        .to_string()
    }

    fn get_body_register() -> (String, String) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        (
            serde_json::json!({
                "email": format!("inconnu{}@example.com", now),
                "password": "mauvais-mot-de-passe",
                "username": "inconnu",
                "fullname": "inconnu"
            })
            .to_string(),
            format!("inconnu{}@example.com", now),
        )
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = construct_app().await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_file_missing_auth() {
        let app = construct_app().await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/files")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_upload_missing_auth() {
        let app = construct_app().await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/files")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_stat_missing_auth() {
        let app = construct_app().await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/stats")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_failed_no_registered() {
        let app = construct_app().await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("Content-Type", "application/json")
                    .body(Body::from(get_body_login(None)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_register_success() {
        let app = construct_app().await.unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("Content-Type", "application/json")
                    .body(Body::from(get_body_register().0))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_register_failed_existed() {
        let app = construct_app().await.unwrap();
        let body = get_body_register().0;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_success() {
        let app = construct_app().await.unwrap();
        let body = get_body_register();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/register")
                    .header("Content-Type", "application/json")
                    .body(Body::from(body.0))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("Content-Type", "application/json")
                    .body(Body::from(get_body_login(Some(body.1))))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        let token = json["token"].as_str().unwrap();
        assert!(!token.is_empty());
    }
}
