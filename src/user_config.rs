use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::AppError;

const USER_CONFIG_RELATIVE_PATH: &[&str] = &["pseq", "config.toml"];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UserConfig {
    store: Option<String>,
}

pub(crate) fn default_store_path() -> Result<Option<PathBuf>, AppError> {
    let Some(config_path) = user_config_path() else {
        return Ok(None);
    };

    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(AppError::ReadFile {
                path: config_path,
                source,
            });
        }
    };

    let config = toml::from_str::<UserConfig>(&content)
        .map_err(|error| invalid_user_config(&config_path, error.to_string()))?;

    let Some(store) = config.store else {
        return Ok(None);
    };

    if store.trim().is_empty() {
        return Err(invalid_user_config(
            &config_path,
            "store path must not be empty".to_owned(),
        ));
    }

    let store_path = PathBuf::from(store);
    if store_path.is_absolute() {
        Ok(Some(store_path))
    } else {
        let base = config_path.parent().unwrap_or_else(|| Path::new(""));
        Ok(Some(base.join(store_path)))
    }
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    env_path("HOME").or_else(|| env_path("USERPROFILE"))
}

fn user_config_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Some(join_relative_config_path(PathBuf::from(path)));
    }

    home_dir().map(|home| join_relative_config_path(home.join(".config")))
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn join_relative_config_path(mut path: PathBuf) -> PathBuf {
    for component in USER_CONFIG_RELATIVE_PATH {
        path.push(component);
    }
    path
}

fn invalid_user_config(path: &Path, message: String) -> AppError {
    AppError::InvalidUserConfig {
        path: path.to_path_buf(),
        message,
    }
}
