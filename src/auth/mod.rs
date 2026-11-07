use std::sync::OnceLock;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use rand::RngCore;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{api::AppState, db::Db, error::AppError};

pub const SESSION_COOKIE: &str = "crm_session";
pub const SESSION_DAYS: i64 = 30;
pub const API_KEY_PREFIX: &str = "crm_";

/// SQL expression for the current UTC time in the format stored in every timestamp column.
pub const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%SZ','now')";

const MIN_PASSWORD: usize = 8;
const MAX_PASSWORD: usize = 128;

#[derive(Clone, Debug, Serialize)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub is_admin: bool,
}

/// Extractor that also requires the caller to be an admin.
pub struct AdminUser(pub AuthUser);

pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Returns a random 256-bit token as hex, with `prefix` prepended.
pub fn new_token(prefix: &str) -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{prefix}{hex}")
}

/// Tokens are stored hashed, so a leaked database does not leak usable sessions or keys.
pub fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn normalize_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_lowercase();
    let valid = email.len() <= 254
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && domain.contains('.'));
    if valid {
        Ok(email)
    } else {
        Err(AppError::BadRequest("invalid email address".into()))
    }
}

pub fn validate_password(password: &str) -> Result<(), AppError> {
    let len = password.chars().count();
    if (MIN_PASSWORD..=MAX_PASSWORD).contains(&len) {
        Ok(())
    } else {
        Err(AppError::BadRequest(format!(
            "password must be {MIN_PASSWORD} to {MAX_PASSWORD} characters"
        )))
    }
}

/// Hashing is CPU heavy, so it runs off the async threads.
pub async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let mut salt_bytes = [0u8; 16];
        rand::rng().fill_bytes(&mut salt_bytes);
        let salt = SaltString::encode_b64(&salt_bytes).map_err(|e| AppError::Internal(e.to_string()))?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

pub async fn verify_password(hash: String, password: String) -> bool {
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash)
            .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

/// A hash to verify against when the email is unknown, so login timing does not reveal which emails exist.
pub async fn dummy_hash() -> Result<String, AppError> {
    static HASH: OnceLock<String> = OnceLock::new();
    if let Some(hash) = HASH.get() {
        return Ok(hash.clone());
    }
    let hash = hash_password("not-a-real-password".into()).await?;
    Ok(HASH.get_or_init(|| hash).clone())
}

/// Creates a session for `user_id` and returns the raw token. Only its hash is stored.
pub async fn create_session(db: &std::sync::Arc<Db>, user_id: &str) -> Result<String, AppError> {
    let token = new_token("");
    let (id, user_id) = (token_hash(&token), user_id.to_string());
    db.write(move |c| {
        // Opportunistic cleanup, so expired rows do not pile up.
        c.execute(&format!("DELETE FROM sessions WHERE expires_at <= {NOW}"), [])?;
        c.execute(
            &format!(
                "INSERT INTO sessions (id, user_id, expires_at, created_at)
                 VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ','now','+{SESSION_DAYS} days'), {NOW})"
            ),
            rusqlite::params![id, user_id],
        )?;
        Ok(())
    })
    .await?;
    Ok(token)
}

/// Finds the raw session token in the `Cookie` header.
pub fn session_token(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.to_string())
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(str::trim)
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let row = |r: &rusqlite::Row| {
            Ok(AuthUser {
                id: r.get(0)?,
                email: r.get(1)?,
                is_admin: r.get::<_, i64>(2)? != 0,
            })
        };

        if let Some(key) = bearer_token(parts) {
            if !key.starts_with(API_KEY_PREFIX) {
                return Err(AppError::Unauthorized);
            }
            let hash = token_hash(key);
            let h = hash.clone();
            let user = state
                .db
                .read(move |c| {
                    let mut stmt = c.prepare(
                        "SELECT u.id, u.email, u.is_admin FROM api_keys k
                         JOIN users u ON u.id = k.user_id WHERE k.key_hash = ?1",
                    )?;
                    let mut rows = stmt.query_map([&h], row)?;
                    Ok(rows.next().transpose()?)
                })
                .await?
                .ok_or(AppError::Unauthorized)?;
            // Record usage at most once an hour, to keep reads from turning into writes.
            state
                .db
                .write(move |c| {
                    c.execute(
                        &format!(
                            "UPDATE api_keys SET last_used_at = {NOW}
                             WHERE key_hash = ?1 AND (last_used_at IS NULL
                                OR last_used_at < strftime('%Y-%m-%dT%H:%M:%SZ','now','-1 hour'))"
                        ),
                        [&hash],
                    )?;
                    Ok(())
                })
                .await?;
            return Ok(user);
        }

        let token = session_token(parts).ok_or(AppError::Unauthorized)?;
        let id = token_hash(&token);
        state
            .db
            .read(move |c| {
                let mut stmt = c.prepare(&format!(
                    "SELECT u.id, u.email, u.is_admin FROM sessions s
                     JOIN users u ON u.id = s.user_id
                     WHERE s.id = ?1 AND s.expires_at > {NOW}"
                ))?;
                let mut rows = stmt.query_map([&id], row)?;
                Ok(rows.next().transpose()?)
            })
            .await?
            .ok_or(AppError::Unauthorized)
    }
}

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, AppError> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.is_admin {
            Ok(AdminUser(user))
        } else {
            Err(AppError::Forbidden("admin access required".into()))
        }
    }
}
