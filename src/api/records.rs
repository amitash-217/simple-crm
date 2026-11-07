use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{ApiJson, ApiQuery, AppState};
use crate::{
    auth::AuthUser,
    error::AppError,
    records::{self, ListParams, Record, RecordList, DEFAULT_LIMIT, MAX_LIMIT},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{table}/records", get(list).post(create))
        .route("/{table}/records/{id}", get(show).patch(update).delete(remove))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListQuery {
    /// Full-text search text.
    q: Option<String>,
    /// JSON filter tree.
    filter: Option<String>,
    /// JSON array of `{field, dir}`.
    sort: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordBody {
    #[serde(default)]
    values: Map<String, Value>,
}

fn parse_json(name: &str, raw: Option<String>) -> Result<Option<Value>, AppError> {
    raw.map(|s| {
        serde_json::from_str(&s).map_err(|e| AppError::BadRequest(format!("`{name}` is not valid JSON: {e}")))
    })
    .transpose()
}

async fn list(
    _: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
    ApiQuery(query): ApiQuery<ListQuery>,
) -> Result<Json<RecordList>, AppError> {
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(AppError::BadRequest(format!("limit must be between 1 and {MAX_LIMIT}")));
    }
    let params = ListParams {
        q: query.q,
        filter: parse_json("filter", query.filter)?,
        sort: parse_json("sort", query.sort)?,
        limit,
        cursor: query.cursor,
    };
    Ok(Json(state.db.read(move |c| records::list(c, &table, params)).await?))
}

async fn create(
    user: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
    ApiJson(body): ApiJson<RecordBody>,
) -> Result<(StatusCode, Json<Record>), AppError> {
    let record = state
        .db
        .write(move |c| records::create(c, &table, &user.id, &body.values))
        .await?;
    Ok((StatusCode::CREATED, Json(record)))
}

async fn show(
    _: AuthUser,
    State(state): State<AppState>,
    Path((table, id)): Path<(String, String)>,
) -> Result<Json<Record>, AppError> {
    Ok(Json(state.db.read(move |c| records::get(c, &table, &id)).await?))
}

async fn update(
    user: AuthUser,
    State(state): State<AppState>,
    Path((table, id)): Path<(String, String)>,
    ApiJson(body): ApiJson<RecordBody>,
) -> Result<Json<Record>, AppError> {
    let record = state
        .db
        .write(move |c| records::update(c, &table, &id, &user.id, &body.values))
        .await?;
    Ok(Json(record))
}

async fn remove(
    _: AuthUser,
    State(state): State<AppState>,
    Path((table, id)): Path<(String, String)>,
) -> Result<Json<Value>, AppError> {
    state.db.write(move |c| records::delete(c, &table, &id)).await?;
    Ok(Json(json!({ "ok": true })))
}
