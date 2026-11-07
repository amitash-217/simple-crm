use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tower::ServiceExt;

use super::{router, AppState};
use crate::db::Db;

pub(super) struct Reply {
    pub status: StatusCode,
    pub cookie: Option<String>,
    pub body: Value,
}

pub(super) fn app() -> (Router, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(&dir.path().join("test.db")).unwrap();
    (router(AppState { db, cookie_secure: false }), dir)
}

/// `auth` is a full header: `("cookie", "crm_session=...")` or `("authorization", "Bearer ...")`.
pub(super) async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    auth: Option<(&str, &str)>,
) -> Reply {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some((name, value)) = auth {
        req = req.header(name, value);
    }
    let req = match body {
        Some(b) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let cookie = res
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().split(';').next().unwrap().to_string());
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Reply { status, cookie, body }
}

pub(super) async fn register_admin(app: &Router) -> String {
    let r = call(
        app,
        Method::POST,
        "/api/auth/register",
        Some(json!({ "email": "Admin@Example.com", "password": "correct horse" })),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    r.cookie.expect("session cookie")
}

#[tokio::test]
async fn status_reports_setup_needed_until_first_user() {
    let (app, _dir) = app();
    let before = call(&app, Method::GET, "/api/auth/status", None, None).await;
    assert_eq!(before.status, StatusCode::OK);
    assert_eq!(before.body["needs_setup"], true);
    register_admin(&app).await;
    let after = call(&app, Method::GET, "/api/auth/status", None, None).await;
    assert_eq!(after.body["needs_setup"], false);
}

#[tokio::test]
async fn first_user_registers_as_admin_and_registration_then_closes() {
    let (app, _dir) = app();
    let cookie = register_admin(&app).await;

    let me = call(&app, Method::GET, "/api/auth/me", None, Some(("cookie", &cookie))).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["email"], "admin@example.com");
    assert_eq!(me.body["is_admin"], true);

    let second = call(
        &app,
        Method::POST,
        "/api/auth/register",
        Some(json!({ "email": "b@example.com", "password": "another pass" })),
        None,
    )
    .await;
    assert_eq!(second.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn me_requires_auth() {
    let (app, _dir) = app();
    let r = call(&app, Method::GET, "/api/auth/me", None, None).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = call(&app, Method::GET, "/api/auth/me", None, Some(("cookie", "crm_session=bogus"))).await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn register_rejects_bad_input() {
    let (app, _dir) = app();
    for body in [
        json!({ "email": "not-an-email", "password": "correct horse" }),
        json!({ "email": "a@example.com", "password": "short" }),
    ] {
        let r = call(&app, Method::POST, "/api/auth/register", Some(body), None).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn login_checks_password_and_logout_ends_session() {
    let (app, _dir) = app();
    register_admin(&app).await;

    let bad = call(
        &app,
        Method::POST,
        "/api/auth/login",
        Some(json!({ "email": "admin@example.com", "password": "wrong password" })),
        None,
    )
    .await;
    assert_eq!(bad.status, StatusCode::UNAUTHORIZED);

    let unknown = call(
        &app,
        Method::POST,
        "/api/auth/login",
        Some(json!({ "email": "nobody@example.com", "password": "correct horse" })),
        None,
    )
    .await;
    assert_eq!(unknown.status, StatusCode::UNAUTHORIZED);

    let ok = call(
        &app,
        Method::POST,
        "/api/auth/login",
        Some(json!({ "email": " ADMIN@example.com ", "password": "correct horse" })),
        None,
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK);
    let cookie = ok.cookie.unwrap();

    let out = call(&app, Method::POST, "/api/auth/logout", None, Some(("cookie", &cookie))).await;
    assert_eq!(out.status, StatusCode::OK);
    let me = call(&app, Method::GET, "/api/auth/me", None, Some(("cookie", &cookie))).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_creates_users_and_members_cannot_manage_them() {
    let (app, _dir) = app();
    let admin = register_admin(&app).await;

    let created = call(
        &app,
        Method::POST,
        "/api/users",
        Some(json!({ "email": "member@example.com", "password": "member pass" })),
        Some(("cookie", &admin)),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED);
    assert_eq!(created.body["is_admin"], false);

    let dup = call(
        &app,
        Method::POST,
        "/api/users",
        Some(json!({ "email": "MEMBER@example.com", "password": "member pass" })),
        Some(("cookie", &admin)),
    )
    .await;
    assert_eq!(dup.status, StatusCode::CONFLICT);

    let login = call(
        &app,
        Method::POST,
        "/api/auth/login",
        Some(json!({ "email": "member@example.com", "password": "member pass" })),
        None,
    )
    .await;
    let member = login.cookie.unwrap();

    let list = call(&app, Method::GET, "/api/users", None, Some(("cookie", &member))).await;
    assert_eq!(list.status, StatusCode::FORBIDDEN);
    let list = call(&app, Method::GET, "/api/users", None, Some(("cookie", &admin))).await;
    assert_eq!(list.body.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn last_admin_cannot_be_demoted_or_deleted() {
    let (app, _dir) = app();
    let admin = register_admin(&app).await;
    let me = call(&app, Method::GET, "/api/auth/me", None, Some(("cookie", &admin))).await;
    let id = me.body["id"].as_str().unwrap().to_string();

    let demote = call(
        &app,
        Method::PATCH,
        &format!("/api/users/{id}"),
        Some(json!({ "is_admin": false })),
        Some(("cookie", &admin)),
    )
    .await;
    assert_eq!(demote.status, StatusCode::CONFLICT);

    let delete = call(&app, Method::DELETE, &format!("/api/users/{id}"), None, Some(("cookie", &admin))).await;
    assert_eq!(delete.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn password_change_signs_the_user_out() {
    let (app, _dir) = app();
    let admin = register_admin(&app).await;
    let me = call(&app, Method::GET, "/api/auth/me", None, Some(("cookie", &admin))).await;
    let id = me.body["id"].as_str().unwrap().to_string();

    let patch = call(
        &app,
        Method::PATCH,
        &format!("/api/users/{id}"),
        Some(json!({ "password": "brand new pass" })),
        Some(("cookie", &admin)),
    )
    .await;
    assert_eq!(patch.status, StatusCode::OK);

    let me = call(&app, Method::GET, "/api/auth/me", None, Some(("cookie", &admin))).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);

    let login = call(
        &app,
        Method::POST,
        "/api/auth/login",
        Some(json!({ "email": "admin@example.com", "password": "brand new pass" })),
        None,
    )
    .await;
    assert_eq!(login.status, StatusCode::OK);
}

#[tokio::test]
async fn api_key_authenticates_until_deleted() {
    let (app, _dir) = app();
    let cookie = register_admin(&app).await;

    let created = call(
        &app,
        Method::POST,
        "/api/api-keys",
        Some(json!({ "name": "script" })),
        Some(("cookie", &cookie)),
    )
    .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let key = created.body["key"].as_str().unwrap().to_string();
    let key_id = created.body["id"].as_str().unwrap().to_string();
    assert!(key.starts_with("crm_"));

    let bearer = format!("Bearer {key}");
    let me = call(&app, Method::GET, "/api/auth/me", None, Some(("authorization", &bearer))).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["email"], "admin@example.com");

    // The list never exposes the raw key.
    let list = call(&app, Method::GET, "/api/api-keys", None, Some(("cookie", &cookie))).await;
    assert_eq!(list.body[0]["name"], "script");
    assert!(list.body[0].get("key").is_none());
    assert!(list.body[0]["last_used_at"].is_string());

    let del = call(&app, Method::DELETE, &format!("/api/api-keys/{key_id}"), None, Some(("cookie", &cookie))).await;
    assert_eq!(del.status, StatusCode::OK);
    let me = call(&app, Method::GET, "/api/auth/me", None, Some(("authorization", &bearer))).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bearer_token_without_key_prefix_is_rejected() {
    let (app, _dir) = app();
    register_admin(&app).await;
    let me = call(&app, Method::GET, "/api/auth/me", None, Some(("authorization", "Bearer abc"))).await;
    assert_eq!(me.status, StatusCode::UNAUTHORIZED);
}
