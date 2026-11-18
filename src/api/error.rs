use crate::{api::state::AppState, db::File, files_vault_errors::FileVaultError};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
impl IntoResponse for FileVaultError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            FileVaultError::HashMismatch => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            FileVaultError::MissingPayload | FileVaultError::PacketCorrupted => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            FileVaultError::FileOpeningError(_) => (StatusCode::NOT_FOUND, self.to_string()),
            FileVaultError::ConnectionError(_) | FileVaultError::TcpSendingError(_) => {
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub async fn details_file(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<File>, FileVaultError> {
    let file = sqlx::query_as!(File, "SELECT * FROM files WHERE id = $1", id as i32)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| FileVaultError::FileOpeningError(e.to_string()))?
        .ok_or(FileVaultError::FileOpeningError("introuvable".into()))?;

    Ok(Json(file))
}
