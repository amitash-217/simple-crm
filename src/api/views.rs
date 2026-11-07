use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{ApiJson, AppState};
use crate::{
    auth::AuthUser,
    error::AppError,
    views::{self, NewView, ViewOut, ViewPatch},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{table}/views", get(list).post(create))
        .route("/{table}/views/{view}", get(show).patch(update).delete(remove))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewViewBody {
    name: String,
    kind: String,
    #[serde(default = "empty_config")]
    config: Value,
}

fn empty_config() -> Value {
    json!({})
}

/// `kind` is fixed once a view exists, so it is not accepted here.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewPatchBody {
    name: Option<String>,
    config: Option<Value>,
    position: Option<i64>,
}

async fn list(
    _: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
) -> Result<Json<Vec<ViewOut>>, AppError> {
    Ok(Json(state.db.read(move |c| views::list(c, &table)).await?))
}

async fn show(
    _: AuthUser,
    State(state): State<AppState>,
    Path((table, view)): Path<(String, String)>,
) -> Result<Json<ViewOut>, AppError> {
    Ok(Json(state.db.read(move |c| views::get(c, &table, &view)).await?))
}

async fn create(
    user: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
    ApiJson(body): ApiJson<NewViewBody>,
) -> Result<(StatusCode, Json<ViewOut>), AppError> {
    let new = NewView { name: body.name, kind: body.kind, config: body.config };
    let view = state.db.write(move |c| views::create(c, &user.id, &table, new)).await?;
    Ok((StatusCode::CREATED, Json(view)))
}

async fn update(
    _: AuthUser,
    State(state): State<AppState>,
    Path((table, view)): Path<(String, String)>,
    ApiJson(body): ApiJson<ViewPatchBody>,
) -> Result<Json<ViewOut>, AppError> {
    let patch = ViewPatch { name: body.name, config: body.config, position: body.position };
    Ok(Json(state.db.write(move |c| views::update(c, &table, &view, patch)).await?))
}

async fn remove(
    _: AuthUser,
    State(state): State<AppState>,
    Path((table, view)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    state.db.write(move |c| views::delete(c, &table, &view)).await?;
    Ok(Json(json!({ "ok": true })))
}
