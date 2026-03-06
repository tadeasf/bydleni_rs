use std::env;

use anyhow::{Context, Result};

/// Application configuration loaded from environment variables.
#[derive(Clone, Debug)]
pub struct Config {
    pub fred_api_key: String,
    pub database_url: String,
    pub server_port: u16,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Requires `DATABASE_URL` to be set. `FRED_API_KEY` and `SERVER_PORT`
    /// have sensible defaults for development.
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            fred_api_key: env::var("FRED_API_KEY").unwrap_or_default(),
            database_url: env::var("DATABASE_URL")
                .context("DATABASE_URL must be set (e.g. sqlite:bydleni.db)")?,
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("SERVER_PORT must be a valid u16")?,
        })
    }
}
