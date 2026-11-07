use axum::{
    extract::{Path, State},
    routing::{get, patch},
    Json, Router,
};
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{ApiJson, AppState};
use crate::{
    auth::{self, AdminUser, NOW},
    error::AppError,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", patch(update).delete(remove))
}

#[derive(Serialize)]
struct UserOut {
    id: String,
    email: String,
    is_admin: bool,
    created_at: String,
}

async fn list(_: AdminUser, State(state): State<AppState>) -> Result<Json<Vec<UserOut>>, AppError> {
    let users = state
        .db
        .read(|c| {
            let mut stmt =
                c.prepare("SELECT id, email, is_admin, created_at FROM users ORDER BY created_at")?;
            let rows = stmt.query_map([], |r| {
                Ok(UserOut {
                    id: r.get(0)?,
                    email: r.get(1)?,
                    is_admin: r.get::<_, i64>(2)? != 0,
                    created_at: r.get(3)?,
                })
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
        .await?;
    Ok(Json(users))
}

#[derive(Deserialize)]
struct CreateUser {
    email: String,
    password: String,
    #[serde(default)]
    is_admin: bool,
}

async fn create(
    _: AdminUser,
    State(state): State<AppState>,
    ApiJson(body): ApiJson<CreateUser>,
) -> Result<(axum::http::StatusCode, Json<UserOut>), AppError> {
    let email = auth::normalize_email(&body.email)?;
    auth::validate_password(&body.password)?;
    let hash = auth::hash_password(body.password).await?;
    let id = auth::new_id();

    let (uid, e, is_admin) = (id.clone(), email.clone(), body.is_admin);
    let created_at = state
        .db
        .write(move |c| {
            let tx = c.transaction()?;
            let taken: i64 =
                tx.query_row("SELECT COUNT(*) FROM users WHERE email = ?1", [&e], |r| r.get(0))?;
            if taken > 0 {
                return Err(AppError::Conflict("a user with that email already exists".into()));
            }
            tx.execute(
                &format!(
                    "INSERT INTO users (id, email, password_hash, is_admin, created_at)
                     VALUES (?1, ?2, ?3, ?4, {NOW})"
                ),
                params![uid, e, hash, is_admin],
            )?;
            let created_at: String =
                tx.query_row("SELECT created_at FROM users WHERE id = ?1", [&uid], |r| r.get(0))?;
            tx.commit()?;
            Ok(created_at)
        })
        .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(UserOut { id, email, is_admin: body.is_admin, created_at }),
    ))
}

#[derive(Deserialize)]
struct UpdateUser {
    password: Option<String>,
    is_admin: Option<bool>,
}

/// The workspace must always keep at least one admin, or nobody can manage accounts.
fn ensure_other_admin(tx: &Transaction, id: &str) -> Result<(), AppError> {
    let others: i64 = tx.query_row(
        "SELECT COUNT(*) FROM users WHERE is_admin = 1 AND id != ?1",
        [id],
        |r| r.get(0),
    )?;
    if others == 0 {
        return Err(AppError::Conflict("cannot remove the last admin".into()));
    }
    Ok(())
}

async fn update(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(body): ApiJson<UpdateUser>,
) -> Result<Json<serde_json::Value>, AppError> {
    let hash = match body.password {
        Some(password) => {
            auth::validate_password(&password)?;
            Some(auth::hash_password(password).await?)
        }
        None => None,
    };
    let is_admin = body.is_admin;

    state
        .db
        .write(move |c| {
            let tx = c.transaction()?;
            let was_admin: Option<i64> = tx
                .query_row("SELECT is_admin FROM users WHERE id = ?1", [&id], |r| r.get(0))
                .ok();
            let was_admin = was_admin.ok_or(AppError::NotFound)? != 0;
            if let Some(make_admin) = is_admin {
                if was_admin && !make_admin {
                    ensure_other_admin(&tx, &id)?;
                }
                tx.execute("UPDATE users SET is_admin = ?1 WHERE id = ?2", params![make_admin, id])?;
            }
            if let Some(hash) = hash {
                tx.execute("UPDATE users SET password_hash = ?1 WHERE id = ?2", params![hash, id])?;
                // A new password signs the user out everywhere.
                tx.execute("DELETE FROM sessions WHERE user_id = ?1", [&id])?;
            }
            tx.commit()?;
            Ok(())
        })
        .await?;
    Ok(Json(json!({ "ok": true })))
}

async fn remove(
    _: AdminUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .db
        .write(move |c| {
            let tx = c.transaction()?;
            let was_admin: Option<i64> = tx
                .query_row("SELECT is_admin FROM users WHERE id = ?1", [&id], |r| r.get(0))
                .ok();
            if was_admin.ok_or(AppError::NotFound)? != 0 {
                ensure_other_admin(&tx, &id)?;
            }
            // Tables and views keep existing without their creator. Their `created_by` is a plain reference with
            // no ON DELETE action, so it must be cleared first or the delete fails.
            tx.execute("UPDATE tables SET created_by = NULL WHERE created_by = ?1", [&id])?;
            tx.execute("UPDATE views SET created_by = NULL WHERE created_by = ?1", [&id])?;
            // Sessions and API keys go with the user through ON DELETE CASCADE.
            tx.execute("DELETE FROM users WHERE id = ?1", [&id])?;
            tx.commit()?;
            Ok(())
        })
        .await?;
    Ok(Json(json!({ "ok": true })))
}
