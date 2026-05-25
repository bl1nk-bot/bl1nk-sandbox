use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub notion_token: String,
    pub notion_database_id: String,
    pub notion_version: String,
    pub vault_path: String,
    /// Subfolder inside vault to scan (optional, default: scan all)
    pub vault_subfolder: Option<String>,
}

pub fn load(path: &str) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Cannot read config file: {}", path))?;
    let cfg: Config = serde_json::from_str(&content)
        .with_context(|| "Failed to parse settings.json")?;
    Ok(cfg)
}
