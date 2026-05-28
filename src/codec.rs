use serde::Serialize;

use crate::error::AppError;
use crate::yaml;

pub(crate) fn encode_json<T: Serialize + ?Sized>(value: &T) -> Result<String, AppError> {
    let mut json =
        serde_json::to_string_pretty(value).map_err(|source| AppError::SerializeJson { source })?;
    json.push('\n');
    Ok(json)
}

pub(crate) fn encode_yaml_frontmatter<T: Serialize + ?Sized>(
    metadata: &T,
    body: &str,
) -> Result<String, AppError> {
    let mut yaml =
        yaml::to_string(metadata).map_err(|source| AppError::SerializeYaml { source })?;
    if let Some(stripped) = yaml.strip_prefix("---\n") {
        yaml = stripped.to_owned();
    }
    if let Some(stripped) = yaml.strip_suffix("...\n") {
        yaml = stripped.to_owned();
    }
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }

    Ok(format!("---\n{yaml}---\n{body}"))
}

pub(crate) fn split_yaml_frontmatter(content: &str) -> Result<(&str, &str), String> {
    let start_len = if content.starts_with("---\n") {
        4
    } else if content.starts_with("---\r\n") {
        5
    } else {
        return Err("missing opening YAML frontmatter delimiter".to_owned());
    };

    let rest = &content[start_len..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let line_without_ending = line.trim_end_matches(['\r', '\n']);
        if line_without_ending == "---" {
            let body_start = offset + line.len();
            return Ok((&rest[..offset], &rest[body_start..]));
        }
        offset += line.len();
    }

    if rest[offset..].trim_end_matches('\r') == "---" {
        return Ok((&rest[..offset], ""));
    }

    Err("missing closing YAML frontmatter delimiter".to_owned())
}
