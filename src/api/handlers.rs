use std::{
    path::Path,
    io::SeekFrom
};
use axum::{
    Extension, Json, body::Body, extract::{
        Multipart, Path as Path_axum, State
    }, http::{HeaderMap, StatusCode, header}, response::{IntoResponse, Response}
};
use serde::{
    Serialize,
    Deserialize
};
use sha2::{
    Sha256,
    Digest
};
use tokio::{
    fs::{self, File as File_tokio}, io::{AsyncWriteExt, AsyncReadExt, AsyncSeekExt}
};

use tokio_util::io::ReaderStream;

use crate::{
    api::{
        auth::{Claims, generate_token}, state::AppState
    }, db::File, files_vault_errors::FileVaultError
};

#[derive(Serialize)]
pub struct Stats {
    total_files: i64,
    total_bytes: i64,
    nb_users: i64,
}

#[derive(Deserialize)]
pub struct InscriptionRequest {
    email: String,
    fullname: String,
    username: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
}


pub async fn list_files(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>
) -> Json<Vec<File>> {
    let user_id = claims.sub;
    let files = sqlx::query_as!(
        File,
        "SELECT * FROM files WHERE user_id = $1 ORDER BY created_at DESC", user_id as i32
    )
    .fetch_all(&state.pool)
    .await
    .unwrap();

    Json(files)
}

pub async fn upload_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart
) -> Result<(StatusCode, Json<File>), FileVaultError> {
    let user_id = claims.sub;
    while let Some(mut field) = multipart.next_field().await
        .map_err(|e| FileVaultError::TcpSendingError(e.to_string()))?
    {
        let original_name = 
            field
            .file_name()
            .ok_or(FileVaultError::MissingPayload)?
            .to_string();
        
        let safe_name =
            Path::new(&original_name)
            .file_name()
            .ok_or(FileVaultError::IncorrectDataType)?
            .to_string_lossy()
            .into_owned();

        let file_path =
            Path::new("server_files")
            .join(&safe_name);

        let mut file_disc = 
            fs::File::create(&file_path)
            .await
            .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;

        let mut hasher = Sha256::new();
        let mut total_size: u64 = 0;
        
        while let Some(chunk) = field.chunk().await
            .map_err(|e| FileVaultError::TcpSendingError(e.to_string()))?
        {
            total_size += chunk.len() as u64;
            if total_size > state.max_upload_bytes {
                let _ = fs::remove_file(&file_path).await;
                return Err(FileVaultError::FileWritingError("quota dépassé".to_string()));
            }
            hasher.update(&chunk);
            
            file_disc
            .write_all(&chunk).await
            .map_err(|e| FileVaultError::FileWritingError(e.to_string()))?;
        }
        
        let sha256 = format!("{:x}", hasher.finalize());
        
        let file = sqlx::query_as!(
            File,
            r#"INSERT INTO files (name, size, sha256, file_path, user_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, size, created_at, user_id, file_path, sha256, nb_downloads
            "#,
            safe_name, total_size as i64, sha256,
            file_path.to_string_lossy().into_owned(), user_id as i32
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| FileVaultError::FileWritingError(e.to_string()))?;
    
        return Ok((StatusCode::CREATED, Json(file)));
    }
    Err(FileVaultError::MissingPayload)
}

// GET /stats
pub async fn statistics(State(state): State<AppState>) -> Json<Stats> {
    let row = sqlx::query!(
        r#"SELECT
        COUNT(*) as "total_files!",
        COALESCE(SUM(size), 0)::BIGINT as "total_bytes!",
        COUNT(DISTINCT user_id) as "nb_users!"
        FROM files"#
    )
    .fetch_one(&state.pool)
    .await
    .unwrap();

    Json(Stats {
        total_files: row.total_files,
        total_bytes: row.total_bytes,
        nb_users: row.nb_users,
    })
}


pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<InscriptionRequest>,
) -> Result<StatusCode, FileVaultError> {
    let hash_pwd = bcrypt::hash(&payload.password, bcrypt::DEFAULT_COST)
    .map_err(|e| FileVaultError::ConnectionError(e.to_string()))?;

    sqlx::query!(
        "INSERT INTO users (email, username, password, fullname) VALUES ($1, $2, $3, $4)",
        payload.email, payload.username, hash_pwd, payload.fullname
    )
    .execute(&state.pool)
    .await
    .map_err(|e| FileVaultError::ConnectionError(format!("email déjà utilisé ? {e}")))?;

    Ok(StatusCode::CREATED)
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, FileVaultError> {
    let user = sqlx::query!(
        "SELECT id, password FROM users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| FileVaultError::ConnectionError(e.to_string()))?
    .ok_or(FileVaultError::ConnectionError("identifiants invalides".into()))?;

    let valid_password = bcrypt::verify(&payload.password, &user.password)
    .unwrap_or(false);

    if !valid_password {
        return Err(FileVaultError::ConnectionError("identifiants invalides".into()));
    }
    
    let token = generate_token(user.id as i64, &state.jwt_secret)?;
    Ok(Json(LoginResponse { token }))
}

pub async fn download_file(
    State(state): State<AppState>,
    Path_axum(id): Path_axum<i32>,
    headers: HeaderMap,
    Extension(claims): Extension<Claims>
) -> Result<Response, FileVaultError> {

    let user_id = claims.sub;

    let meta = sqlx::query_as!(
        File, "SELECT * FROM files WHERE (id = $1 AND user_id = $2)", id, user_id as i32
    )
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?
        .ok_or(FileVaultError::FileOpeningError("introuvable".into()))?;

    let mut file = File_tokio::open(&meta.file_path).await
    .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;

    let total_size = meta.size as u64;
    
    sqlx::query!("UPDATE files SET nb_downloads = nb_downloads + 1 WHERE id = $1", id)
    .execute(&state.pool).await.ok();

    if let Some(range) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        let (start, end) = parser_range(range, total_size)?;
        let length = end - start + 1;

        file.seek(SeekFrom::Start(start)).await
        .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?;
    
        let stream = ReaderStream::new(file.take(length));
        let body = Body::from_stream(stream);
        
        return Ok((
            StatusCode::PARTIAL_CONTENT,
            [
                (header::CONTENT_RANGE, format!("bytes {start}-{end}/{total_size}")),
                (header::ACCEPT_RANGES, "bytes".to_string()),
                (header::CONTENT_LENGTH, length.to_string()),
            ],
            body,
        ).into_response());
    }
    
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::ACCEPT_RANGES, "bytes".to_string()),
            (header::CONTENT_LENGTH, total_size.to_string()),
        ],
        body,
    ).into_response())
}


fn parser_range(range: &str, total_size: u64) -> Result<(u64, u64), FileVaultError> {
    let bytes_part = range.strip_prefix("bytes=")
    .ok_or(FileVaultError::IncorrectDataType)?;

    let mut parts = bytes_part.splitn(2, '-');
    let start: u64 = parts.next().unwrap_or("0").parse()
    .map_err(|_| FileVaultError::IncorrectDataType)?;

    let end: u64 = match parts.next().filter(|s| !s.is_empty()) {
        Some(s) => s.parse().map_err(|_| FileVaultError::IncorrectDataType)?,
        None => total_size - 1,
    };
    
    Ok((start, end.min(total_size - 1)))
}

pub async fn delete_file(
    State(state): State<AppState>,
    Path_axum(id): Path_axum<i32>,
    Extension(claims): Extension<Claims>
) -> Result<StatusCode, FileVaultError> {
    let user_id = claims.sub;

    sqlx::query!(
        r#"DELETE FROM files WHERE (id = $1 AND user_id = $2)"#, id, user_id as i32
    )
    .execute(&state.pool)
    .await
    .map_err(|e| FileVaultError::ConnectionError(format!("Not find ? {e}")))?;

    Ok(StatusCode::OK)
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "tls": true }))
}