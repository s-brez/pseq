use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::store;

pub(crate) const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigFile {
    pub version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_runner: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub runners: BTreeMap<String, RunnerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunnerConfig {
    pub first: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ConfigShowOutput {
    pub path: String,
    pub version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_runner: Option<String>,
    pub runner_count: usize,
}

pub fn show(store_path: &Path) -> Result<ConfigShowOutput, AppError> {
    store::require_valid_store(store_path)?;

    let config = read_config(store_path)?;

    Ok(ConfigShowOutput {
        path: CONFIG_FILE.to_owned(),
        version: config.version,
        default_runner: config.default_runner,
        runner_count: config.runners.len(),
    })
}

pub(crate) fn read_config(store_path: &Path) -> Result<ConfigFile, AppError> {
    let path = config_path(store_path);
    let content = fs::read_to_string(&path).map_err(|source| AppError::ReadFile {
        path: path.clone(),
        source,
    })?;
    parse_config(&content).map_err(|message| AppError::InvalidConfig { path, message })
}

pub(crate) fn write_config(store_path: &Path, config: &ConfigFile) -> Result<(), AppError> {
    validate_config(config)?;
    let path = config_path(store_path);
    let content = encode_config(config)?;
    fs::write(&path, content).map_err(|source| AppError::WriteFile { path, source })
}

pub(crate) fn parse_config(content: &str) -> Result<ConfigFile, String> {
    let config = toml::from_str::<ConfigFile>(content).map_err(|error| error.to_string())?;
    validate_config(&config).map_err(|error| error.to_string())?;
    Ok(config)
}

pub(crate) fn encode_config(config: &ConfigFile) -> Result<String, AppError> {
    let mut content =
        toml::to_string_pretty(config).map_err(|source| AppError::SerializeToml { source })?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    Ok(content)
}

pub(crate) fn config_path(store_path: &Path) -> std::path::PathBuf {
    store_path.join(CONFIG_FILE)
}

pub(crate) fn config_pathspec() -> std::path::PathBuf {
    std::path::PathBuf::from(CONFIG_FILE)
}

pub(crate) fn validate_runner_name(name: &str) -> Result<(), AppError> {
    if is_valid_runner_name(name) {
        Ok(())
    } else {
        Err(AppError::InvalidRunnerName {
            name: name.to_owned(),
        })
    }
}

fn validate_config(config: &ConfigFile) -> Result<(), AppError> {
    if config.version != 1 {
        return Err(AppError::InvalidConfig {
            path: std::path::PathBuf::from(CONFIG_FILE),
            message: format!("unsupported config version {}; expected 1", config.version),
        });
    }

    if let Some(default_runner) = &config.default_runner {
        validate_runner_name(default_runner)?;
        if !config.runners.contains_key(default_runner) {
            return Err(AppError::InvalidConfig {
                path: std::path::PathBuf::from(CONFIG_FILE),
                message: format!("default runner {default_runner:?} is not defined in config file"),
            });
        }
    }

    for (name, runner) in &config.runners {
        validate_runner_name(name)?;
        validate_argv(&runner.first, &format!("runner {name:?} first command"))?;
        if let Some(next) = &runner.next {
            validate_argv(next, &format!("runner {name:?} next command"))?;
        }
    }

    Ok(())
}

pub(crate) fn validate_argv(argv: &[String], label: &str) -> Result<(), AppError> {
    if argv
        .first()
        .is_some_and(|command| !command.trim().is_empty())
    {
        Ok(())
    } else {
        Err(AppError::RunnerCommandEmpty {
            context: label.to_owned(),
        })
    }
}

fn is_valid_runner_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    (first.is_ascii_alphanumeric() || first == '_')
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}
