use sqlx::{postgres::PgPoolOptions, Pool, Postgres};


#[derive(Debug, sqlx::FromRow)]
pub struct File {
    pub id: i64,
    pub name: String,
    pub size: i64,
}

pub async fn connexion_db() -> Result<Pool<Postgres>, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL doit être définie");

    PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
}

pub async fn get_all_files(pool: Pool<Postgres>) -> Result<Vec<File>, sqlx::Error> {
    sqlx::query_as!(File,
        "SELECT id, name, size
        FROM files
        "
    )
    .fetch_all(&pool)
    .await
}

pub async fn insert_file(pool: Pool<Postgres>, name: &str, size: u64) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let id = sqlx::query!(
        "INSERT INTO files (name, size)
        VALUES ($1, $2) RETURNING id",
        name, size as i64
    )
    .fetch_one(&mut *tx)
    .await?
    .id;
    println!("[SERVER] File inserted with id={id}");
    tx.commit().await?;

    Ok(())
}

pub async fn delete_file(pool: Pool<Postgres>, id: i32) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "DELETE FROM files
        WHERE id = $1 RETURNING id",
        id
    )
    .fetch_one(&mut *tx)
    .await?
    .id;
    println!("[SERVER]: File deleted with id={id}");
    tx.commit().await?;
    Ok(())
}
