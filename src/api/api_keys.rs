use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{ApiJson, AppState};
use crate::{
    auth::{self, AuthUser, API_KEY_PREFIX, NOW},
    error::AppError,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", delete(remove))
}

#[derive(Serialize)]
struct KeyOut {
    id: String,
    name: String,
    created_at: String,
    last_used_at: Option<String>,
}

async fn list(user: AuthUser, State(state): State<AppState>) -> Result<Json<Vec<KeyOut>>, AppError> {
    let keys = state
        .db
        .read(move |c| {
            let mut stmt = c.prepare(
                "SELECT id, name, created_at, last_used_at FROM api_keys
                 WHERE user_id = ?1 ORDER BY created_at",
            )?;
            let rows = stmt.query_map([&user.id], |r| {
                Ok(KeyOut {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    created_at: r.get(2)?,
                    last_used_at: r.get(3)?,
                })
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
        .await?;
    Ok(Json(keys))
}

#[derive(Deserialize)]
struct CreateKey {
    name: String,
}

/// The raw key appears in this response only. The database keeps its hash.
async fn create(
    user: AuthUser,
    State(state): State<AppState>,
    ApiJson(body): ApiJson<CreateKey>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest("name must be 1 to 100 characters".into()));
    }
    let key = auth::new_token(API_KEY_PREFIX);
    let (id, hash, n) = (auth::new_id(), auth::token_hash(&key), name.clone());
    let key_id = id.clone();
    state
        .db
        .write(move |c| {
            c.execute(
                &format!(
                    "INSERT INTO api_keys (id, user_id, name, key_hash, created_at)
                     VALUES (?1, ?2, ?3, ?4, {NOW})"
                ),
                params![id, user.id, n, hash],
            )?;
            Ok(())
        })
        .await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": key_id, "name": name, "key": key }))))
}

async fn remove(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let removed = state
        .db
        .write(move |c| {
            Ok(c.execute("DELETE FROM api_keys WHERE id = ?1 AND user_id = ?2", params![id, user.id])?)
        })
        .await?;
    if removed == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(json!({ "ok": true })))
}
