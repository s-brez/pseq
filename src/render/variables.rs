use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::AppError;
use crate::yaml;

pub(crate) fn load_variables(
    variables_file: Option<&Path>,
    variable_assignments: &[String],
) -> Result<BTreeMap<String, String>, AppError> {
    let mut variables = BTreeMap::new();

    if let Some(path) = variables_file {
        variables.extend(read_variables_file(path)?);
    }

    for assignment in variable_assignments {
        let (name, value) =
            assignment
                .split_once('=')
                .ok_or_else(|| AppError::InvalidVariableAssignment {
                    assignment: assignment.clone(),
                })?;
        validate_variable_name(name)?;
        let value = if let Some(path) = value.strip_prefix('@') {
            if path.is_empty() {
                return Err(AppError::InvalidVariableAssignment {
                    assignment: assignment.clone(),
                });
            }
            let path = Path::new(path);
            fs::read_to_string(path).map_err(|source| AppError::ReadFile {
                path: path.to_path_buf(),
                source,
            })?
        } else {
            value.to_owned()
        };
        variables.insert(name.to_owned(), value);
    }

    Ok(variables)
}

fn read_variables_file(path: &Path) -> Result<BTreeMap<String, String>, AppError> {
    let content = fs::read_to_string(path).map_err(|source| AppError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;

    let variables = match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => parse_json_variables(path, &content)?,
        Some("toml") => parse_toml_variables(path, &content)?,
        Some("yaml" | "yml") => parse_yaml_variables(path, &content)?,
        _ => parse_untyped_variables(path, &content)?,
    };

    for name in variables.keys() {
        validate_variable_name(name)?;
    }

    Ok(variables)
}

fn parse_json_variables(path: &Path, content: &str) -> Result<BTreeMap<String, String>, AppError> {
    serde_json::from_str(content).map_err(|source| invalid_variables_file(path, source))
}

fn parse_toml_variables(path: &Path, content: &str) -> Result<BTreeMap<String, String>, AppError> {
    toml::from_str(content).map_err(|source| invalid_variables_file(path, source))
}

fn parse_yaml_variables(path: &Path, content: &str) -> Result<BTreeMap<String, String>, AppError> {
    yaml::from_str(content).map_err(|source| invalid_variables_file(path, source))
}

fn parse_untyped_variables(
    path: &Path,
    content: &str,
) -> Result<BTreeMap<String, String>, AppError> {
    parse_json_variables(path, content)
        .or_else(|_| parse_toml_variables(path, content))
        .or_else(|_| parse_yaml_variables(path, content))
        .map_err(|source| {
            let message = format!(
                "{source}; variables files must be JSON, TOML, or YAML mappings of string keys to string values",
            );
            invalid_variables_file(path, message)
        })
}

fn invalid_variables_file(path: &Path, source: impl ToString) -> AppError {
    AppError::InvalidVariablesFile {
        path: path.to_path_buf(),
        message: source.to_string(),
    }
}

pub(crate) fn validate_variable_name(name: &str) -> Result<(), AppError> {
    if is_valid_variable_name(name) {
        Ok(())
    } else {
        Err(AppError::InvalidVariableName {
            name: name.to_owned(),
        })
    }
}

pub(super) fn is_valid_variable_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}
