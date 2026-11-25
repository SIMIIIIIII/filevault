use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

use crate::files_vault_errors::FileVaultError;

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize, Clone)]
pub struct File {
    pub id: i32,
    pub name: String,
    pub size: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<i32>,
    pub file_path: String,
    pub sha256: String,
    pub nb_downloads: i64,
}

pub struct CheckUser {
    pub id: i32,
    pub password: String,
}

pub async fn connexion_db(url: Option<String>) -> Result<Pool<Postgres>, sqlx::Error> {
    let database_url = match url {
        Some(url) => url,
        None => std::env::var("DATABASE_URL").expect("DATABASE_URL doit être définie"),
    };

    PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
}

pub async fn get_all_files(
    pool: Pool<Postgres>,
    user_id: i32,
    limit: i64,
    page: i64,
) -> Result<Vec<File>, sqlx::Error> {
    let new_page = if page <= 0 { 1 } else { page };
    let new_limit = if limit <= 0 { 20 } else { limit.min(100) };

    let offset = (new_page - 1) * new_limit;

    sqlx::query_as!(
        File,
        "SELECT *
        FROM files
        WHERE user_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        ",
        user_id,
        new_limit,
        offset
    )
    .fetch_all(&pool)
    .await
}

pub async fn get_file(
    pool: Pool<Postgres>,
    user_id: i32,
    file_id: i32,
) -> Result<File, sqlx::Error> {
    sqlx::query_as!(
        File,
        "SELECT *
        FROM files
        WHERE id = $1
        AND user_id = $2
        ",
        file_id,
        user_id
    )
    .fetch_one(&pool)
    .await
}

pub async fn update_nb_download(
    pool: Pool<Postgres>,
    user_id: i32,
    file_id: i32,
) -> Result<i64, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let nb_downloads = sqlx::query_scalar::<_, i64>(
        "UPDATE files
        SET nb_downloads = nb_downloads + 1
        WHERE id = $1
        AND user_id = $2
        RETURNING nb_downloads",
    )
    .bind(file_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(nb_downloads)
}

pub async fn insert_file(
    pool: Pool<Postgres>,
    name: String,
    size: i64,
    sha256: String,
    file_path: String,
    user_id: i32,
) -> Result<File, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let res = sqlx::query!(
        "INSERT INTO files (name, size, sha256, file_path, user_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, sha256, name, size, created_at, file_path, nb_downloads",
        name,
        size as i64,
        sha256,
        file_path,
        user_id
    )
    .fetch_one(&mut *tx)
    .await?;

    println!("[SERVER] File inserted with id={}", res.id);
    tx.commit().await?;

    Ok(File {
        id: res.id,
        name: res.name,
        size: res.size,
        created_at: res.created_at,
        user_id: Some(user_id),
        file_path: res.file_path,
        nb_downloads: res.nb_downloads,
        sha256: res.sha256,
    })
}

pub async fn delete_file(pool: Pool<Postgres>, id: i32, user_id: i32) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "DELETE FROM files
        WHERE id = $1
        AND user_id = $2
        RETURNING id",
        id,
        user_id
    )
    .fetch_one(&mut *tx)
    .await?;
    println!("[SERVER]: File deleted with id={id}");
    tx.commit().await?;
    Ok(())
}

pub async fn check_user(
    pool: Pool<Postgres>,
    email: String,
    password: String,
) -> Result<CheckUser, FileVaultError> {
    let user = sqlx::query_as!(
        CheckUser,
        "SELECT id, password
        FROM users
        WHERE email = $1
        ",
        email
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| FileVaultError::ConnectionError(e.to_string()))?;

    let valid_password = bcrypt::verify(&password, &user.password).unwrap_or(false);

    if !valid_password {
        return Err(FileVaultError::ConnectionError(
            "identifiants invalides".into(),
        ));
    }

    Ok(user)
}

pub async fn new_user(
    pool: Pool<Postgres>,
    username: String,
    fullname: String,
    email: String,
    password: String,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let id = sqlx::query!(
        "INSERT INTO users (username, fullname, email, password)
        VALUES ($1, $2, $3, $4)
        RETURNING id",
        username,
        fullname,
        email,
        password
    )
    .fetch_one(&mut *tx)
    .await?
    .id;

    tx.commit().await?;
    println!("[SERVER]: User created with id={id}");
    Ok(())
}
