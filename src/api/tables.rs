use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use super::{ApiJson, AppState};
use crate::{
    auth::AuthUser,
    catalog::{self, FieldConfig, FieldOut, FieldPatch, NewField, OptionInput, Table, TableDetail},
    error::AppError,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{table}", get(show).patch(rename).delete(remove))
        .route("/{table}/fields", get(list_fields).post(add_field))
        .route("/{table}/fields/{field}", patch(update_field).delete(delete_field))
        .merge(super::records::routes())
        .merge(super::views::routes())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TableBody {
    name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewFieldBody {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
    #[serde(default)]
    config: FieldConfig,
    options: Option<Vec<OptionInput>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldPatchBody {
    name: Option<String>,
    options: Option<Vec<OptionInput>>,
}

async fn list(_: AuthUser, State(state): State<AppState>) -> Result<Json<Vec<Table>>, AppError> {
    Ok(Json(state.db.read(|c| catalog::list_tables(c)).await?))
}

async fn create(
    user: AuthUser,
    State(state): State<AppState>,
    ApiJson(body): ApiJson<TableBody>,
) -> Result<(StatusCode, Json<TableDetail>), AppError> {
    let detail = state.db.write(move |c| catalog::create_table(c, &user.id, &body.name)).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

async fn show(
    _: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
) -> Result<Json<TableDetail>, AppError> {
    Ok(Json(state.db.read(move |c| catalog::get_table(c, &table)).await?))
}

async fn rename(
    _: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
    ApiJson(body): ApiJson<TableBody>,
) -> Result<Json<TableDetail>, AppError> {
    Ok(Json(state.db.write(move |c| catalog::rename_table(c, &table, &body.name)).await?))
}

async fn remove(
    _: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.db.write(move |c| catalog::delete_table(c, &table)).await?;
    Ok(Json(json!({ "ok": true })))
}

async fn list_fields(
    _: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
) -> Result<Json<Vec<FieldOut>>, AppError> {
    Ok(Json(state.db.read(move |c| catalog::list_fields(c, &table)).await?))
}

async fn add_field(
    _: AuthUser,
    State(state): State<AppState>,
    Path(table): Path<String>,
    ApiJson(body): ApiJson<NewFieldBody>,
) -> Result<(StatusCode, Json<FieldOut>), AppError> {
    let new = NewField {
        name: body.name,
        field_type: body.field_type,
        config: body.config,
        options: body.options,
    };
    let field = state.db.write(move |c| catalog::add_field(c, &table, new)).await?;
    Ok((StatusCode::CREATED, Json(field)))
}

async fn update_field(
    _: AuthUser,
    State(state): State<AppState>,
    Path((table, field)): Path<(String, String)>,
    ApiJson(body): ApiJson<FieldPatchBody>,
) -> Result<Json<FieldOut>, AppError> {
    let patch = FieldPatch { name: body.name, options: body.options };
    Ok(Json(state.db.write(move |c| catalog::update_field(c, &table, &field, patch)).await?))
}

async fn delete_field(
    _: AuthUser,
    State(state): State<AppState>,
    Path((table, field)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.db.write(move |c| catalog::delete_field(c, &table, &field)).await?;
    Ok(Json(json!({ "ok": true })))
}
