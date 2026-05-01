use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub teloxide_token: String,
    pub google_credentials_path: String,
    pub spreadsheet_id: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load .env file if present (ignore error if not found)
        let _ = dotenv::dotenv();

        Ok(Config {
            teloxide_token: std::env::var("TELOXIDE_TOKEN")
                .context("TELOXIDE_TOKEN environment variable not set")?,
            google_credentials_path: std::env::var("GOOGLE_CREDENTIALS_PATH")
                .context("GOOGLE_CREDENTIALS_PATH environment variable not set")?,
            spreadsheet_id: std::env::var("SPREADSHEET_ID")
                .context("SPREADSHEET_ID environment variable not set")?,
        })
    }
}
