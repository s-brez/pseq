use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::Path;

use crate::error::AppError;
use crate::render;
use crate::runner::{self, ResolvedRunner};

use super::DEFAULT_FEEDBACK_VARIABLE;
use super::model::OutputMode;
use super::types::RunOptions;

pub(super) fn validate_options(options: &RunOptions<'_>) -> Result<(), AppError> {
    if options.iterations == 0 {
        return Err(AppError::InvalidRunInvocation {
            message: "iterations must be greater than zero".to_owned(),
        });
    }
    if options.runner_name.is_some() && !options.ad_hoc_command.is_empty() {
        return Err(AppError::InvalidRunInvocation {
            message: "runner name and ad hoc command cannot be used together".to_owned(),
        });
    }
    if options.feedback_from.is_none() && options.feedback_var.is_some() {
        return Err(AppError::InvalidRunInvocation {
            message: "feedback variable can only be set with --feedback-from".to_owned(),
        });
    }
    if options.feedback_from.is_none() && options.feedback_seed.is_some() {
        return Err(AppError::InvalidRunInvocation {
            message: "feedback seed can only be set with --feedback-from".to_owned(),
        });
    }

    Ok(())
}

pub(super) fn feedback_variable(options: &RunOptions<'_>) -> Result<Option<String>, AppError> {
    match options.feedback_from {
        Some(_) => {
            let variable = options.feedback_var.unwrap_or(DEFAULT_FEEDBACK_VARIABLE);
            render::validate_variable_name(variable)?;
            Ok(Some(variable.to_owned()))
        }
        None => Ok(None),
    }
}

pub(super) fn resolve_runner(
    store_path: &Path,
    options: &RunOptions<'_>,
) -> Result<ResolvedRunner, AppError> {
    if options.ad_hoc_command.is_empty() {
        runner::resolve_named(store_path, options.runner_name)
    } else {
        runner::resolve_ad_hoc(options.ad_hoc_command.to_vec())
    }
}

pub(super) fn load_base_variables(
    options: &RunOptions<'_>,
    feedback_variable: Option<&str>,
) -> Result<BTreeMap<String, String>, AppError> {
    let base_variables =
        render::load_variables(options.variables_file, options.variable_assignments)?;
    if let Some(variable) = feedback_variable
        && base_variables.contains_key(variable)
    {
        return Err(AppError::InvalidRunInvocation {
            message: format!(
                "feedback variable {variable:?} is controlled by --feedback-from and cannot also be supplied with --var or --vars"
            ),
        });
    }
    Ok(base_variables)
}

pub(super) fn load_feedback_seed(options: &RunOptions<'_>) -> Result<String, AppError> {
    let Some(seed) = options.feedback_seed else {
        return Ok(String::new());
    };
    let Some(source) = seed.strip_prefix('@') else {
        return Ok(seed.to_owned());
    };
    if source.is_empty() {
        return Err(AppError::InvalidRunInvocation {
            message: "feedback seed file path must not be empty".to_owned(),
        });
    }
    if source == "-" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|source| AppError::ReadStdin { source })?;
        return Ok(input);
    }

    std::fs::read_to_string(source).map_err(|source_error| AppError::ReadFile {
        path: Path::new(source).to_path_buf(),
        source: source_error,
    })
}

pub(super) fn output_mode(options: &RunOptions<'_>) -> OutputMode {
    if options.capture_output {
        OutputMode::Capture
    } else if options.feedback_from.is_some() {
        OutputMode::Tee
    } else {
        OutputMode::Inherit
    }
}
