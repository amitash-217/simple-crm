use axum::{
    extract::State,
    http::{header, request::Parts, HeaderValue},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use rusqlite::{params, OptionalExtension};
use serde::Deserialize;
use serde_json::json;

use super::{ApiJson, AppState};
use crate::{
    auth::{self, AuthUser, NOW, SESSION_COOKIE, SESSION_DAYS},
    error::AppError,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(me))
        .route("/status", get(status))
}

/// Lets the login screen choose between first-run setup and sign-in. Reveals only whether any user exists.
async fn status(State(state): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let users: i64 = state
        .db
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?))
        .await?;
    Ok(Json(json!({ "needs_setup": users == 0 })))
}

#[derive(Deserialize)]
struct Credentials {
    email: String,
    password: String,
}

fn session_cookie(state: &AppState, token: &str, max_age: i64) -> HeaderValue {
    let secure = if state.cookie_secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}{secure}"
    ))
    .expect("cookie is ASCII")
}

fn with_session(state: &AppState, token: &str, user: serde_json::Value) -> Response {
    (
        [(header::SET_COOKIE, session_cookie(state, token, SESSION_DAYS * 86_400))],
        Json(user),
    )
        .into_response()
}

/// Allowed only while the database has no users. The first user becomes the admin.
async fn register(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<Credentials>,
) -> Result<Response, AppError> {
    let email = auth::normalize_email(&body.email)?;
    auth::validate_password(&body.password)?;
    let hash = auth::hash_password(body.password).await?;
    let id = auth::new_id();

    let (uid, e) = (id.clone(), email.clone());
    state
        .db
        .write(move |c| {
            let tx = c.transaction()?;
            let users: i64 = tx.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
            if users > 0 {
                return Err(AppError::Forbidden(
                    "registration is closed; ask an admin to create your account".into(),
                ));
            }
            tx.execute(
                &format!(
                    "INSERT INTO users (id, email, password_hash, is_admin, created_at)
                     VALUES (?1, ?2, ?3, 1, {NOW})"
                ),
                params![uid, e, hash],
            )?;
            tx.commit()?;
            Ok(())
        })
        .await?;

    let token = auth::create_session(&state.db, &id).await?;
    Ok(with_session(&state, &token, json!({ "id": id, "email": email, "is_admin": true })))
}

async fn login(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<Credentials>,
) -> Result<Response, AppError> {
    let email = body.email.trim().to_lowercase();
    let lookup = email.clone();
    let found: Option<(String, String, bool)> = state
        .db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT id, password_hash, is_admin FROM users WHERE email = ?1",
                [&lookup],
                |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0)),
            )
            .optional()?)
        })
        .await?;

    // Verify against a dummy hash when the email is unknown, so timing does not leak which emails exist.
    let hash = match &found {
        Some((_, hash, _)) => hash.clone(),
        None => auth::dummy_hash().await?,
    };
    let ok = auth::verify_password(hash, body.password).await;
    let (id, is_admin) = match found {
        Some((id, _, is_admin)) if ok => (id, is_admin),
        _ => return Err(AppError::Unauthorized),
    };

    let token = auth::create_session(&state.db, &id).await?;
    Ok(with_session(&state, &token, json!({ "id": id, "email": email, "is_admin": is_admin })))
}

async fn logout(State(state): State<AppState>, req: axum::extract::Request) -> Result<Response, AppError> {
    let (parts, _): (Parts, _) = req.into_parts();
    if let Some(token) = auth::session_token(&parts) {
        let id = auth::token_hash(&token);
        state
            .db
            .write(move |c| {
                c.execute("DELETE FROM sessions WHERE id = ?1", [&id])?;
                Ok(())
            })
            .await?;
    }
    Ok(([(header::SET_COOKIE, session_cookie(&state, "", 0))], Json(json!({ "ok": true }))).into_response())
}

async fn me(user: AuthUser) -> Json<AuthUser> {
    Json(user)
}
