use std::{net::SocketAddr, path::PathBuf};

pub struct Config {
    pub db_path: PathBuf,
    pub bind: SocketAddr,
    /// Adds the `Secure` flag to the session cookie. Turn on when serving over HTTPS.
    pub cookie_secure: bool,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let db_path = std::env::var("CRM_DB").unwrap_or_else(|_| "crm.db".into()).into();
        let bind = std::env::var("CRM_BIND")
            .unwrap_or_else(|_| "127.0.0.1:3000".into())
            .parse()?;
        let cookie_secure = matches!(
            std::env::var("CRM_COOKIE_SECURE").as_deref(),
            Ok("1") | Ok("true")
        );
        Ok(Self { db_path, bind, cookie_secure })
    }
}
