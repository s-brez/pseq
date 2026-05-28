use std::path::Path;

use serde::Serialize;

use crate::config::{self, RunnerConfig};
use crate::error::AppError;
use crate::git;
use crate::store;
use crate::trust as runner_trust;

#[derive(Debug, Clone, Copy)]
pub enum RunnerSlot {
    First,
    Next,
}

#[derive(Debug, Clone)]
pub struct ResolvedRunner {
    pub name: Option<String>,
    pub first: Vec<String>,
    pub next: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct RunnerSetOutput {
    pub name: String,
    pub slot: String,
    pub command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunnerDefaultOutput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunnerListOutput {
    pub runners: Vec<RunnerSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_runner: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunnerSummary {
    pub name: String,
    pub has_next: bool,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct RunnerShowOutput {
    pub name: String,
    pub first: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<Vec<String>>,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct RunnerRemoveOutput {
    pub name: String,
    pub was_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunnerTrustOutput {
    pub name: String,
}

pub fn set(
    store_path: &Path,
    name: String,
    slot: RunnerSlot,
    command: Vec<String>,
) -> Result<RunnerSetOutput, AppError> {
    store::require_valid_store(store_path)?;
    config::validate_runner_name(&name)?;
    config::validate_argv(&command, &format!("runner {name:?} command"))?;

    let mut config_file = config::read_config(store_path)?;
    match slot {
        RunnerSlot::First => {
            let runner = config_file
                .runners
                .entry(name.clone())
                .or_insert_with(|| RunnerConfig {
                    first: Vec::new(),
                    next: None,
                });
            runner.first = command.clone();
        }
        RunnerSlot::Next => {
            let runner = config_file
                .runners
                .get_mut(&name)
                .ok_or_else(|| AppError::RunnerNotFound { name: name.clone() })?;
            runner.next = Some(command.clone());
        }
    }

    let runner = config_file
        .runners
        .get(&name)
        .ok_or_else(|| AppError::RunnerNotFound { name: name.clone() })?;
    runner_trust::record_runner(store_path, &name, &runner.first, runner.next.as_deref())?;

    config::write_config(store_path, &config_file)?;
    let git_commit = commit_config(store_path, &format!("Configure runner {name}"))?;

    Ok(RunnerSetOutput {
        name,
        slot: runner_slot_name(slot).to_owned(),
        command,
        git_commit,
    })
}

pub fn set_default(store_path: &Path, name: String) -> Result<RunnerDefaultOutput, AppError> {
    store::require_valid_store(store_path)?;
    config::validate_runner_name(&name)?;

    let mut config_file = config::read_config(store_path)?;
    if !config_file.runners.contains_key(&name) {
        return Err(AppError::RunnerNotFound { name });
    }
    config_file.default_runner = Some(name.clone());

    config::write_config(store_path, &config_file)?;
    let git_commit = commit_config(store_path, &format!("Set default runner {name}"))?;

    Ok(RunnerDefaultOutput { name, git_commit })
}

pub fn list(store_path: &Path) -> Result<RunnerListOutput, AppError> {
    store::require_valid_store(store_path)?;
    let config_file = config::read_config(store_path)?;
    let runners = config_file
        .runners
        .iter()
        .map(|(name, runner)| RunnerSummary {
            name: name.clone(),
            has_next: runner.next.is_some(),
            is_default: config_file.default_runner.as_deref() == Some(name),
        })
        .collect();

    Ok(RunnerListOutput {
        runners,
        default_runner: config_file.default_runner,
    })
}

pub fn show(store_path: &Path, name: &str) -> Result<RunnerShowOutput, AppError> {
    store::require_valid_store(store_path)?;
    config::validate_runner_name(name)?;
    let config_file = config::read_config(store_path)?;
    let runner = config_file
        .runners
        .get(name)
        .ok_or_else(|| AppError::RunnerNotFound {
            name: name.to_owned(),
        })?;

    Ok(RunnerShowOutput {
        name: name.to_owned(),
        first: runner.first.clone(),
        next: runner.next.clone(),
        is_default: config_file.default_runner.as_deref() == Some(name),
    })
}

pub fn remove(store_path: &Path, name: &str) -> Result<RunnerRemoveOutput, AppError> {
    store::require_valid_store(store_path)?;
    config::validate_runner_name(name)?;
    let mut config_file = config::read_config(store_path)?;
    if config_file.runners.remove(name).is_none() {
        return Err(AppError::RunnerNotFound {
            name: name.to_owned(),
        });
    }
    let was_default = config_file.default_runner.as_deref() == Some(name);
    if was_default {
        config_file.default_runner = None;
    }

    config::write_config(store_path, &config_file)?;
    let git_commit = commit_config(store_path, &format!("Remove runner {name}"))?;

    Ok(RunnerRemoveOutput {
        name: name.to_owned(),
        was_default,
        git_commit,
    })
}

pub fn trust(store_path: &Path, name: &str) -> Result<RunnerTrustOutput, AppError> {
    store::require_valid_store(store_path)?;
    config::validate_runner_name(name)?;
    let config_file = config::read_config(store_path)?;
    let runner = config_file
        .runners
        .get(name)
        .ok_or_else(|| AppError::RunnerNotFound {
            name: name.to_owned(),
        })?;
    runner_trust::record_runner(store_path, name, &runner.first, runner.next.as_deref())?;

    Ok(RunnerTrustOutput {
        name: name.to_owned(),
    })
}

pub fn resolve_named(store_path: &Path, name: Option<&str>) -> Result<ResolvedRunner, AppError> {
    let config_file = config::read_config(store_path)?;
    let name = match name {
        Some(name) => name.to_owned(),
        None => config_file
            .default_runner
            .clone()
            .ok_or(AppError::DefaultRunnerMissing)?,
    };
    config::validate_runner_name(&name)?;
    let runner = config_file
        .runners
        .get(&name)
        .ok_or_else(|| AppError::RunnerNotFound { name: name.clone() })?;
    runner_trust::ensure_runner_trusted(store_path, &name, &runner.first, runner.next.as_deref())?;

    Ok(ResolvedRunner {
        name: Some(name),
        first: runner.first.clone(),
        next: runner.next.clone(),
    })
}

pub fn resolve_ad_hoc(command: Vec<String>) -> Result<ResolvedRunner, AppError> {
    config::validate_argv(&command, "ad hoc runner command")?;
    Ok(ResolvedRunner {
        name: None,
        first: command,
        next: None,
    })
}

impl ResolvedRunner {
    pub(crate) fn command_for_turn(&self, index: usize) -> &[String] {
        if index == 1 {
            &self.first
        } else {
            self.next.as_ref().unwrap_or(&self.first)
        }
    }

    pub(crate) fn label(&self) -> String {
        self.name.clone().unwrap_or_else(|| "ad-hoc".to_owned())
    }
}

fn runner_slot_name(slot: RunnerSlot) -> &'static str {
    match slot {
        RunnerSlot::First => "first",
        RunnerSlot::Next => "next",
    }
}

fn commit_config(store_path: &Path, message: &str) -> Result<Option<String>, AppError> {
    git::commit_paths_if_changed(store_path, &[config::config_pathspec()], message)
}
