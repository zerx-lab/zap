use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

const API_KEY_ENV: &str = "WARP_AI_API_KEY";
const BASE_URL_ENV: &str = "WARP_AI_BASE_URL";
const MODEL_ENV: &str = "WARP_AI_MODEL";

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o";

#[derive(Debug, Deserialize)]
struct ConfigFile {
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
}

pub struct Config {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
}

impl Config {
    pub fn resolve(
        cli_api_key: Option<&str>,
        cli_base_url: Option<&str>,
        cli_model: Option<&str>,
    ) -> Result<Self> {
        let file = load_config_file().unwrap_or_else(|e| {
            log::debug!("No config file loaded: {e}");
            None
        });

        let api_key = cli_api_key
            .map(|s| s.to_owned())
            .or_else(|| std::env::var(API_KEY_ENV).ok())
            .or_else(|| file.as_ref().and_then(|f| f.api_key.clone()));

        let base_url = cli_base_url
            .map(|s| s.to_owned())
            .or_else(|| std::env::var(BASE_URL_ENV).ok())
            .or_else(|| file.as_ref().and_then(|f| f.base_url.clone()))
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());

        let model = cli_model
            .map(|s| s.to_owned())
            .or_else(|| std::env::var(MODEL_ENV).ok())
            .or_else(|| file.as_ref().and_then(|f| f.model.clone()))
            .unwrap_or_else(|| DEFAULT_MODEL.to_owned());

        Ok(Config {
            api_key,
            base_url,
            model,
        })
    }
}

fn config_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".config/warp-ai/config.json"))
}

fn load_config_file() -> Result<Option<ConfigFile>> {
    let path = config_file_path().context("could not determine config file path")?;
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let config: ConfigFile =
        serde_json::from_str(&content).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(config))
}
