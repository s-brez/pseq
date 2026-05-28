use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;

pub(super) fn prepare_runner_command(argv: &[String]) -> Result<Vec<String>, AppError> {
    let current_dir = env::current_dir().map_err(|source| AppError::CurrentDir { source })?;
    let Some(adapter) = RunnerCommandAdapter::detect(argv) else {
        return Ok(argv.to_vec());
    };

    Ok(adapter.prepare(argv, &current_dir))
}

#[derive(Debug, Clone, Copy)]
enum RunnerCommandAdapter {
    Codex,
}

impl RunnerCommandAdapter {
    fn detect(argv: &[String]) -> Option<Self> {
        if is_command_name(argv.first()?, "codex") && argv.get(1).is_some_and(|arg| arg == "exec") {
            return Some(Self::Codex);
        }
        None
    }

    fn prepare(self, argv: &[String], current_dir: &Path) -> Vec<String> {
        match self {
            Self::Codex => prepare_codex_exec(argv, current_dir),
        }
    }
}

fn prepare_codex_exec(argv: &[String], current_dir: &Path) -> Vec<String> {
    if codex_exec_has_sandbox_escape(argv) || codex_exec_sandbox_mode(argv) == Some("read-only") {
        return argv.to_vec();
    }

    let workspace_dir = codex_exec_workspace_dir(argv, current_dir);
    let Some(git_dirs) = git_metadata_dirs(&workspace_dir) else {
        return argv.to_vec();
    };

    let mut command = Vec::with_capacity(argv.len() + git_dirs.len() * 2);
    command.extend_from_slice(&argv[..2]);
    for git_dir in git_dirs {
        if !has_add_dir(argv, &git_dir, &workspace_dir) {
            command.push("--add-dir".to_owned());
            command.push(path_arg(&git_dir));
        }
    }
    command.extend_from_slice(&argv[2..]);
    command
}

fn codex_exec_has_sandbox_escape(argv: &[String]) -> bool {
    argv.iter()
        .any(|arg| arg == "--dangerously-bypass-approvals-and-sandbox")
}

fn codex_exec_sandbox_mode(argv: &[String]) -> Option<&str> {
    let mut mode = None;
    let mut index = 2;
    while index < argv.len() {
        let arg = &argv[index];
        if arg == "--sandbox" || arg == "-s" {
            if let Some(value) = argv.get(index + 1) {
                mode = Some(value.as_str());
            }
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--sandbox=") {
            mode = Some(value);
        }
        index += 1;
    }
    mode
}

fn codex_exec_workspace_dir(argv: &[String], current_dir: &Path) -> PathBuf {
    let mut workspace_dir = current_dir.to_path_buf();
    let mut index = 2;
    while index < argv.len() {
        let arg = &argv[index];
        if matches!(arg.as_str(), "resume" | "review" | "help" | "-" | "--") {
            break;
        }
        if arg == "-C" || arg == "--cd" {
            if let Some(value) = argv.get(index + 1) {
                workspace_dir = resolve_command_path(value, current_dir);
            }
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--cd=") {
            workspace_dir = resolve_command_path(value, current_dir);
        }
        index += 1;
    }
    workspace_dir
}

fn git_metadata_dirs(workspace_dir: &Path) -> Option<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(workspace_dir)
        .args([
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut dirs: Vec<PathBuf> = Vec::new();
    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let path = PathBuf::from(line);
        if !dirs
            .iter()
            .any(|existing| same_command_path(existing, &path))
        {
            dirs.push(path);
        }
    }
    (!dirs.is_empty()).then_some(dirs)
}

fn has_add_dir(argv: &[String], git_dir: &Path, workspace_dir: &Path) -> bool {
    let mut index = 0;
    while index < argv.len() {
        let arg = &argv[index];
        if arg == "--add-dir" {
            if argv
                .get(index + 1)
                .map(|value| resolve_command_path(value, workspace_dir))
                .is_some_and(|path| same_command_path(&path, git_dir))
            {
                return true;
            }
            index += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--add-dir=")
            && same_command_path(&resolve_command_path(value, workspace_dir), git_dir)
        {
            return true;
        }
        index += 1;
    }
    false
}

fn resolve_command_path(value: &str, base: &Path) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn same_command_path(left: &Path, right: &Path) -> bool {
    comparable_path(left) == comparable_path(right)
}

fn comparable_path(path: &Path) -> String {
    path.components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .replace('\\', "/")
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn is_command_name(command: &str, expected: &str) -> bool {
    let Some(file_name) = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
    else {
        return false;
    };
    let command_name = file_name.strip_suffix(".exe").unwrap_or(file_name);
    command_name.eq_ignore_ascii_case(expected)
}
