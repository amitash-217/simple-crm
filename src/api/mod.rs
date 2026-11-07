use std::sync::Arc;

use axum::{
    extract::Request,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use rust_embed::RustEmbed;
use serde_json::json;

use crate::{db::Db, error::AppError};

mod api_keys;
mod auth;
mod docs;
#[cfg(test)]
mod auth_tests;
#[cfg(test)]
mod links_tests;
mod records;
#[cfg(test)]
mod records_tests;
mod search;
#[cfg(test)]
mod search_tests;
mod tables;
#[cfg(test)]
mod tables_tests;
mod users;
mod views;
#[cfg(test)]
mod views_tests;

/// `axum::extract::Query`, but rejections use `AppError`.
#[derive(axum::extract::FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(AppError))]
pub struct ApiQuery<T>(pub T);

/// `axum::Json` for request bodies, but rejections use `AppError`.
#[derive(axum::extract::FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
pub struct ApiJson<T>(pub T);

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Assets;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub cookie_secure: bool,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .nest("/api/auth", auth::routes())
        .nest("/api/users", users::routes())
        .nest("/api/api-keys", api_keys::routes())
        .nest("/api/tables", tables::routes())
        .nest("/api/search", search::routes())
        .merge(docs::routes())
        .fallback(static_files)
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Serves embedded frontend files. Unknown non-API paths get `index.html` so the SPA router works.
async fn static_files(req: Request) -> Response {
    let uri: &Uri = req.uri();
    if uri.path().starts_with("/api/") {
        return (StatusCode::NOT_FOUND, Json(json!({ "error": { "code": "not_found", "message": "not found" } })))
            .into_response();
    }
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let (name, file) = match Assets::get(path) {
        Some(file) => (path, file),
        None => match Assets::get("index.html") {
            Some(file) => ("index.html", file),
            None => return StatusCode::NOT_FOUND.into_response(),
        },
    };
    let mime = mime_guess::from_path(name).first_or_octet_stream();
    ([(header::CONTENT_TYPE, mime.as_ref().to_string())], file.data).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn app() -> (Router, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("test.db")).unwrap();
        (router(AppState { db, cookie_secure: false }), dir)
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let (app, _dir) = app();
        let res = app
            .oneshot(Request::get("/api/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_api_path_is_json_404() {
        let (app, _dir) = app();
        let res = app
            .oneshot(Request::get("/api/nope").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_page_falls_back_to_index() {
        let (app, _dir) = app();
        let res = app
            .oneshot(Request::get("/tables/abc").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
    }
}
