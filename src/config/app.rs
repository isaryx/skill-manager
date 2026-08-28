use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::home_dir;
use crate::error::SkmError;

pub const APP_CONFIG_VERSION: u32 = 1;
pub const APP_CONFIG_FILENAME: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub version: u32,
    pub store: AppStoreSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppStoreSection {
    pub path: PathBuf,
}

pub fn app_config_dir() -> PathBuf {
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !config_home.is_empty() {
            return PathBuf::from(config_home).join("skm");
        }
    }

    directories::BaseDirs::new()
        .map(|dirs| dirs.config_dir().join("skm"))
        .unwrap_or_else(|| home_dir().join(".config").join("skm"))
}

pub fn app_config_path() -> PathBuf {
    app_config_dir().join(APP_CONFIG_FILENAME)
}

pub fn default_app_config(store_path: &Path) -> AppConfig {
    AppConfig {
        version: APP_CONFIG_VERSION,
        store: AppStoreSection {
            path: store_path.to_path_buf(),
        },
    }
}

pub fn read_app_config() -> Result<AppConfig, SkmError> {
    let path = app_config_path();
    let content = fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SkmError::AppConfigNotFound(path.clone())
        } else {
            SkmError::Io(e)
        }
    })?;
    let config: AppConfig = toml::from_str(&content).map_err(|e| SkmError::InvalidAppConfig {
        path: path.clone(),
        message: e.to_string(),
    })?;
    Ok(config)
}

pub fn try_read_app_config() -> Option<AppConfig> {
    read_app_config().ok()
}

pub fn write_app_config(store_path: &Path) -> Result<PathBuf, SkmError> {
    let path = app_config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let config = default_app_config(store_path);
    let content = toml::to_string_pretty(&config)?;
    fs::write(&path, content)?;
    Ok(path)
}

pub fn app_store_path(config: &AppConfig) -> PathBuf {
    config.store.path.clone()
}
