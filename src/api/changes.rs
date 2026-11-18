//! Lets open pages refresh when data changes, whether the write came from the app or the API.
//! Every successful write bumps a version number. A page long-polls `/api/changes` and reloads when it moves.
use std::time::Duration;

use axum::{
    extract::{Request, State},
    http::Method,
    middleware::Next,
    response::Response,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::{ApiQuery, AppState};
use crate::auth::AuthUser;

/// How long a poll waits for a change before it answers with the same version.
const WAIT: Duration = Duration::from_secs(25);

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/changes", get(poll))
}

#[derive(Deserialize)]
struct PollQuery {
    since: Option<u64>,
}

/// Answers with the current version at once if it differs from `since`, else when it next changes or after `WAIT`.
async fn poll(_: AuthUser, State(state): State<AppState>, ApiQuery(q): ApiQuery<PollQuery>) -> Json<Value> {
    let mut rx = state.changes.subscribe();
    if q.since == Some(*rx.borrow_and_update()) {
        let _ = tokio::time::timeout(WAIT, rx.changed()).await;
    }
    let version = *rx.borrow();
    Json(json!({ "version": version }))
}

/// Middleware: bumps the version after every successful write request.
pub async fn track(State(state): State<AppState>, req: Request, next: Next) -> Response {
    // A search is a POST that reads. Counting it would also make a page's own reloads loop.
    let write = !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) && !req.uri().path().ends_with("/search");
    let res = next.run(req).await;
    if write && res.status().is_success() {
        state.changes.send_modify(|v| *v += 1);
    }
    res
}

#[cfg(test)]
mod tests {
    use axum::http::{Method, StatusCode};
    use serde_json::json;

    use super::super::auth_tests::{app, call, register_admin};

    #[tokio::test]
    async fn writes_bump_the_version_and_reads_do_not() {
        let (app, _dir) = app();
        let cookie = register_admin(&app).await;
        let auth = Some(("cookie", cookie.as_str()));

        let r = call(&app, Method::GET, "/api/changes", None, None).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);

        let start = call(&app, Method::GET, "/api/changes", None, auth).await.body["version"].as_u64().unwrap();
        call(&app, Method::GET, "/api/tables", None, auth).await;
        let created = call(&app, Method::POST, "/api/tables", Some(json!({ "name": "Deals" })), auth).await;
        assert_eq!(created.status, StatusCode::CREATED);
        call(&app, Method::POST, "/api/tables/Deals/records/search", Some(json!({})), auth).await;

        // `since` differs from the current version, so the poll answers at once.
        let now = call(&app, Method::GET, &format!("/api/changes?since={start}"), None, auth).await;
        assert_eq!(now.body["version"].as_u64(), Some(start + 1));
    }
}
