use crate::db;

    // Handler GET /fichiers/:id avec cache
    async fn get_fichier(
        State(state): State<AppState>,
        Path(id): Path<i64>,
    ) -> Result<Json<FichierCache>, AppError> {

        // 1. Chercher dans le cache Redis
        if let Some(cached) = state.cache.write().await.get(id).await? {
            tracing::debug!(id, "Cache HIT");
            return Ok(Json(cached));
        }

        // 2. Cache MISS → PostgreSQL
        tracing::debug!(id,    "Cache MISS");

        let fichier = db::get_file(pool, user_id, file_id)?;

        // 3. Peupler le cache
        state.cache.write().await.set(&fichier).await?;
        Ok(Json(fichier))
    }