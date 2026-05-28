use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use uuid::Uuid;

use crate::error::AppError;

pub(crate) fn edit_text(initial: &str, extension: &str) -> Result<Option<String>, AppError> {
    let editor = editor_command()?;
    let (dir, path) = create_temp_edit_file(initial, extension)?;

    let status = run_editor(&editor, &path);
    if let Err(error) = &status {
        let _ = fs::remove_dir_all(&dir);
        return Err(AppError::EditorSpawn {
            editor,
            source: io_error_clone(error),
        });
    }

    let status = status.expect("handled Err above");
    if !status.success() {
        let _ = fs::remove_dir_all(&dir);
        return Err(AppError::EditorFailed {
            editor,
            status: status
                .code()
                .map(|code| format!("exit code {code}"))
                .unwrap_or_else(|| "terminated by signal".to_owned()),
        });
    }

    let edited = fs::read_to_string(&path).map_err(|source| AppError::ReadFile {
        path: path.clone(),
        source,
    })?;
    let _ = fs::remove_dir_all(&dir);

    if edited == initial {
        Ok(None)
    } else {
        Ok(Some(edited))
    }
}

fn editor_command() -> Result<String, AppError> {
    env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .ok_or(AppError::EditorUnavailable)
}

fn create_temp_edit_file(initial: &str, extension: &str) -> Result<(PathBuf, PathBuf), AppError> {
    let dir = env::temp_dir().join(format!("pseq-edit-{}", Uuid::new_v4().simple()));
    create_private_dir(&dir)?;
    let path = dir.join(format!("edit.{}", extension.trim_start_matches('.')));
    write_private_file(&path, initial)?;
    Ok((dir, path))
}

#[cfg(unix)]
fn create_private_dir(path: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::DirBuilderExt;

    fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(|source| AppError::CreateDir {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
fn create_private_dir(path: &Path) -> Result<(), AppError> {
    fs::create_dir(path).map_err(|source| AppError::CreateDir {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(unix)]
fn write_private_file(path: &Path, content: &str) -> Result<(), AppError> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|source| AppError::WriteFile {
            path: path.to_path_buf(),
            source,
        })?;
    file.write_all(content.as_bytes())
        .map_err(|source| AppError::WriteFile {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
fn write_private_file(path: &Path, content: &str) -> Result<(), AppError> {
    fs::write(path, content).map_err(|source| AppError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(windows)]
fn run_editor(editor: &str, path: &Path) -> std::io::Result<std::process::ExitStatus> {
    use std::os::windows::process::CommandExt;

    let command = format!("{editor} {}", shell_quote(path));
    Command::new("cmd").arg("/C").raw_arg(command).status()
}

#[cfg(not(windows))]
fn run_editor(editor: &str, path: &Path) -> std::io::Result<std::process::ExitStatus> {
    let command = format!("{editor} {}", shell_quote(path));
    Command::new("sh").arg("-c").arg(command).status()
}

fn shell_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    if cfg!(windows) {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn io_error_clone(error: &std::io::Error) -> std::io::Error {
    std::io::Error::new(error.kind(), error.to_string())
}
