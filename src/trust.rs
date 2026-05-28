use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::paths;

const TRUST_RELATIVE_PATH: &[&str] = &["pseq", "trusted-runners.toml"];
const TRUST_LOCK_ATTEMPTS: usize = 100;
const TRUST_LOCK_RETRY: Duration = Duration::from_millis(10);

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedRunnersFile {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    runners: Vec<TrustedRunner>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrustedRunner {
    store: String,
    name: String,
    first: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next: Option<Vec<String>>,
}

pub(crate) fn ensure_runner_trusted(
    store_path: &Path,
    name: &str,
    first: &[String],
    next: Option<&[String]>,
) -> Result<(), AppError> {
    let record = trusted_runner(store_path, name, first, next);
    let file = read_trusted_runners()?;
    if file.runners.iter().any(|trusted| trusted == &record) {
        return Ok(());
    }

    Err(AppError::RunnerNotTrusted {
        name: name.to_owned(),
        store: record.store,
    })
}

pub(crate) fn record_runner(
    store_path: &Path,
    name: &str,
    first: &[String],
    next: Option<&[String]>,
) -> Result<(), AppError> {
    let record = trusted_runner(store_path, name, first, next);
    update_trusted_runners(|file| {
        file.runners
            .retain(|trusted| trusted.store != record.store || trusted.name != record.name);
        file.runners.push(record);
    })
}

fn trusted_runner(
    store_path: &Path,
    name: &str,
    first: &[String],
    next: Option<&[String]>,
) -> TrustedRunner {
    TrustedRunner {
        store: paths::display(
            &store_path
                .canonicalize()
                .unwrap_or_else(|_| store_path.to_path_buf()),
        ),
        name: name.to_owned(),
        first: first.to_owned(),
        next: next.map(<[String]>::to_owned),
    }
}

fn read_trusted_runners() -> Result<TrustedRunnersFile, AppError> {
    let path = trust_file_path()?;
    read_trusted_runners_from_path(&path)
}

fn update_trusted_runners(update: impl FnOnce(&mut TrustedRunnersFile)) -> Result<(), AppError> {
    let path = trust_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    with_trust_lock(&path, || {
        let mut file = read_trusted_runners_from_path(&path)?;
        update(&mut file);
        write_trusted_runners_to_path(&path, &file)
    })
}

fn read_trusted_runners_from_path(path: &Path) -> Result<TrustedRunnersFile, AppError> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(TrustedRunnersFile::default());
        }
        Err(source) => {
            return Err(AppError::ReadFile {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    toml::from_str(&content).map_err(|error| AppError::InvalidUserConfig {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn write_trusted_runners_to_path(path: &Path, file: &TrustedRunnersFile) -> Result<(), AppError> {
    let content =
        toml::to_string_pretty(file).map_err(|source| AppError::SerializeToml { source })?;
    let temp_path = path.with_extension(format!("toml.{}.tmp", std::process::id()));
    fs::write(&temp_path, content).map_err(|source| AppError::WriteFile {
        path: temp_path.clone(),
        source,
    })?;
    replace_file(&temp_path, path)
}

fn replace_file(from: &Path, to: &Path) -> Result<(), AppError> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) if cfg!(windows) && to.exists() => {
            fs::remove_file(to).map_err(|source| AppError::RemoveFile {
                path: to.to_path_buf(),
                source,
            })?;
            fs::rename(from, to).map_err(|source| AppError::MoveFile {
                from: from.to_path_buf(),
                to: to.to_path_buf(),
                source,
            })
        }
        Err(source) => Err(AppError::MoveFile {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
            source,
        }),
    }
}

fn with_trust_lock<T>(
    trust_file_path: &Path,
    operation: impl FnOnce() -> Result<T, AppError>,
) -> Result<T, AppError> {
    let lock_path = trust_file_path.with_extension("lock");
    for _ in 0..TRUST_LOCK_ATTEMPTS {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_lock) => {
                let result = operation();
                let _ = fs::remove_file(&lock_path);
                return result;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                thread::sleep(TRUST_LOCK_RETRY);
            }
            Err(source) => {
                return Err(AppError::WriteFile {
                    path: lock_path,
                    source,
                });
            }
        }
    }

    Err(AppError::RunnerTrustLocked { path: lock_path })
}

fn trust_file_path() -> Result<PathBuf, AppError> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty()) {
        return Ok(join_trust_relative_path(PathBuf::from(path)));
    }

    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .ok_or(AppError::RunnerTrustUnavailable)?;
    Ok(join_trust_relative_path(
        PathBuf::from(home).join(".config"),
    ))
}

fn join_trust_relative_path(mut path: PathBuf) -> PathBuf {
    for component in TRUST_RELATIVE_PATH {
        path.push(component);
    }
    path
}
