use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::error::AppError;
use crate::paths;

#[derive(Debug)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub head: Option<String>,
    pub dirty: bool,
    pub changed_paths: usize,
    pub untracked_paths: usize,
    pub error: Option<String>,
}

#[derive(Debug)]
pub struct GitLogEntry {
    pub commit: String,
    pub short_commit: String,
    pub author_name: String,
    pub author_email: String,
    pub timestamp: String,
    pub summary: String,
}

#[derive(Debug)]
pub struct GitStatusEntry {
    pub status: String,
    pub path: String,
}

pub fn require_available() -> Result<(), AppError> {
    let output = Command::new("git")
        .arg("--version")
        .output()
        .map_err(|source| AppError::GitSpawn { source })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(AppError::GitFailed {
            command: "git --version".to_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

pub fn ensure_repo(path: &Path) -> Result<(), AppError> {
    if worktree_root(path)?
        .as_deref()
        .is_some_and(|root| paths::same_path(root, path))
    {
        return Ok(());
    }

    checked(path, &["init", "--quiet"], false)?;
    Ok(())
}

pub fn commit_paths_if_changed(
    repo_path: &Path,
    paths: &[PathBuf],
    message: &str,
) -> Result<Option<String>, AppError> {
    let mut pathspecs = paths
        .iter()
        .map(|path| normalize_pathspec(repo_path, path))
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    pathspecs.sort();
    pathspecs.dedup();

    if pathspecs.is_empty() {
        return Ok(None);
    }

    let mut add_args = vec!["add".to_owned(), "--all".to_owned(), "--".to_owned()];
    add_args.extend(pathspecs.iter().cloned());
    checked(repo_path, &add_args, false)?;

    let mut diff_args = vec![
        "diff".to_owned(),
        "--cached".to_owned(),
        "--quiet".to_owned(),
        "--exit-code".to_owned(),
        "--".to_owned(),
    ];
    diff_args.extend(pathspecs.iter().cloned());
    let diff = raw(repo_path, &diff_args, false)?;
    if diff.status.success() {
        return Ok(None);
    }

    let hooks_path = disabled_hooks_path()?;
    let mut commit_args = vec![
        "-c".to_owned(),
        "user.name=pseq".to_owned(),
        "-c".to_owned(),
        "user.email=pseq@example.invalid".to_owned(),
        "-c".to_owned(),
        format!("core.hooksPath={}", paths::display(&hooks_path)),
        "commit".to_owned(),
        "--quiet".to_owned(),
        "--no-verify".to_owned(),
        "-m".to_owned(),
        message.to_owned(),
        "--only".to_owned(),
        "--".to_owned(),
    ];
    commit_args.extend(pathspecs);
    checked(repo_path, &commit_args, false)?;

    head_short(repo_path)
}

fn disabled_hooks_path() -> Result<PathBuf, AppError> {
    let path = env::temp_dir().join("pseq-empty-git-hooks");
    fs::create_dir_all(&path).map_err(|source| AppError::CreateDir {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

pub fn unstage_paths(repo_path: &Path, paths: &[PathBuf]) -> Result<(), AppError> {
    let mut pathspecs = paths
        .iter()
        .map(|path| normalize_pathspec(repo_path, path))
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    pathspecs.sort();
    pathspecs.dedup();

    if pathspecs.is_empty() {
        return Ok(());
    }

    let mut args = vec!["reset".to_owned(), "--quiet".to_owned(), "--".to_owned()];
    args.extend(pathspecs);
    checked(repo_path, &args, false)?;
    Ok(())
}

pub fn worktree_root(path: &Path) -> Result<Option<PathBuf>, AppError> {
    let output = raw(path, &["rev-parse", "--show-toplevel"], true)?;
    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let root = stdout.trim();
    if root.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(root)))
    }
}

pub fn status(path: &Path) -> GitStatus {
    let branch = command_text(path, &["branch", "--show-current"]);
    let head = match head_short(path) {
        Ok(value) => value,
        Err(error) => {
            return GitStatus {
                branch: branch.ok().flatten(),
                head: None,
                dirty: false,
                changed_paths: 0,
                untracked_paths: 0,
                error: Some(error.to_string()),
            };
        }
    };

    let porcelain = command_text(path, &["status", "--porcelain=v1", "--untracked-files=all"]);
    match porcelain {
        Ok(Some(text)) => {
            let lines: Vec<&str> = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect();
            let untracked_paths = lines.iter().filter(|line| line.starts_with("??")).count();

            GitStatus {
                branch: branch.ok().flatten(),
                head,
                dirty: !lines.is_empty(),
                changed_paths: lines.len(),
                untracked_paths,
                error: None,
            }
        }
        Ok(None) => GitStatus {
            branch: branch.ok().flatten(),
            head,
            dirty: false,
            changed_paths: 0,
            untracked_paths: 0,
            error: None,
        },
        Err(error) => GitStatus {
            branch: branch.ok().flatten(),
            head,
            dirty: false,
            changed_paths: 0,
            untracked_paths: 0,
            error: Some(error.to_string()),
        },
    }
}

pub fn log_entries(path: &Path) -> Result<Vec<GitLogEntry>, AppError> {
    let args = [
        "log",
        "--date-order",
        "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%cI%x1f%s%x1e",
    ];
    let output = raw(path, &args, true)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if stderr.contains("does not have any commits yet") {
            return Ok(Vec::new());
        }
        return Err(AppError::GitFailed {
            command: format_command(path, args.iter().copied()),
            stderr,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    for record in stdout.split('\x1e') {
        let record = record.trim_matches('\n');
        if record.is_empty() {
            continue;
        }

        let fields = record.split('\x1f').collect::<Vec<_>>();
        if fields.len() != 6 {
            return Err(AppError::GitFailed {
                command: format_command(path, args.iter().copied()),
                stderr: "unexpected git log output".to_owned(),
            });
        }

        entries.push(GitLogEntry {
            commit: fields[0].to_owned(),
            short_commit: fields[1].to_owned(),
            author_name: fields[2].to_owned(),
            author_email: fields[3].to_owned(),
            timestamp: fields[4].to_owned(),
            summary: fields[5].to_owned(),
        });
    }

    Ok(entries)
}

pub fn status_entries(path: &Path) -> Result<Vec<GitStatusEntry>, AppError> {
    let args = ["status", "--porcelain=v1", "--untracked-files=all"];
    let output = checked(path, &args, true)?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let status = line.get(..2).unwrap_or(line).to_owned();
            let path = line.get(3..).unwrap_or_default().to_owned();
            GitStatusEntry { status, path }
        })
        .collect())
}

pub fn diff_patch(path: &Path) -> Result<String, AppError> {
    let args = ["diff", "--no-ext-diff", "--no-color", "HEAD", "--"];
    let output = raw(path, &args, true)?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if stderr.contains("ambiguous argument 'HEAD'") || stderr.contains("bad revision 'HEAD'") {
        let fallback_args = ["diff", "--no-ext-diff", "--no-color", "--"];
        let output = checked(path, &fallback_args, true)?;
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }

    Err(AppError::GitFailed {
        command: format_command(path, args.iter().copied()),
        stderr,
    })
}

pub fn list_files_at_ref(
    path: &Path,
    reference: &str,
    pathspecs: &[&str],
) -> Result<Vec<String>, AppError> {
    let mut args = vec!["ls-tree", "-r", "--name-only", reference, "--"];
    args.extend(pathspecs.iter().copied());
    let output = checked(path, &args, true)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

pub fn show_text_at_ref(
    path: &Path,
    reference: &str,
    store_relative_path: &str,
) -> Result<String, AppError> {
    let spec = format!("{reference}:{store_relative_path}");
    let output = checked(path, &["show", &spec], true)?;
    String::from_utf8(output.stdout).map_err(|source| AppError::GitFileNotUtf8 {
        reference: reference.to_owned(),
        path: store_relative_path.to_owned(),
        source,
    })
}

fn head_short(path: &Path) -> Result<Option<String>, AppError> {
    command_text(path, &["rev-parse", "--short", "HEAD"])
}

fn command_text(path: &Path, args: &[&str]) -> Result<Option<String>, AppError> {
    let output = raw(path, args, true)?;
    if !output.status.success() {
        return Ok(None);
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

fn checked<S>(path: &Path, args: &[S], read_only: bool) -> Result<Output, AppError>
where
    S: AsRef<OsStr> + AsRef<str>,
{
    let output = raw(path, args, read_only)?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(AppError::GitFailed {
            command: format_command(path, args.iter().map(<S as AsRef<str>>::as_ref)),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    }
}

fn raw<S>(path: &Path, args: &[S], read_only: bool) -> Result<Output, AppError>
where
    S: AsRef<OsStr>,
{
    let mut command = Command::new("git");
    command.arg("-C").arg(path);
    command.args(args.iter().map(<S as AsRef<OsStr>>::as_ref));
    if read_only {
        command.env("GIT_OPTIONAL_LOCKS", "0");
    }
    command
        .output()
        .map_err(|source| AppError::GitSpawn { source })
}

fn format_command<'a>(path: &Path, args: impl IntoIterator<Item = &'a str>) -> String {
    let mut parts = vec!["git".to_owned(), "-C".to_owned(), paths::display(path)];
    parts.extend(args.into_iter().map(str::to_owned));
    parts.join(" ")
}

fn normalize_pathspec(repo_path: &Path, path: &Path) -> String {
    let relative = if path.is_absolute() {
        path.strip_prefix(repo_path).unwrap_or(path)
    } else {
        path
    };

    paths::normalize(relative)
}
