use axum::{extract::State, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{ApiQuery, AppState};
use crate::{
    auth::AuthUser,
    error::AppError,
    search::{self, DEFAULT_LIMIT, MAX_LIMIT},
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(search))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchQuery {
    q: String,
    /// Matches per table.
    limit: Option<usize>,
}

/// Full-text search across every table. Results are grouped by table, best match first.
async fn search(
    _: AuthUser,
    State(state): State<AppState>,
    ApiQuery(query): ApiQuery<SearchQuery>,
) -> Result<Json<Value>, AppError> {
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(AppError::BadRequest(format!("limit must be between 1 and {MAX_LIMIT}")));
    }
    let results = state.db.read(move |c| search::global(c, &query.q, limit)).await?;
    Ok(Json(json!({ "results": results })))
}
