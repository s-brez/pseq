use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use uuid::Uuid;

use crate::capture;
use crate::collection;
use crate::config;
use crate::error::AppError;
use crate::fragment;
use crate::git;
use crate::paths;
use crate::render;
use crate::sequence;
use crate::user_config;

const REQUIRED_DIRS: &[&str] = &["fragments", "sequences", "captures", "renders"];
const CONFIG_CONTENT: &str = "version = 1\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitTarget {
    NewOrEmpty,
    ExistingGitRoot,
}

#[derive(Debug, Default)]
struct ScaffoldChanges {
    touched_paths: Vec<PathBuf>,
    created_files: Vec<PathBuf>,
    created_dirs: Vec<PathBuf>,
    created_git_path: Option<PathBuf>,
}

impl ScaffoldChanges {
    fn rollback(&self, store_path: &Path) {
        if !self.touched_paths.is_empty() {
            let _ = git::unstage_paths(store_path, &self.touched_paths);
        }

        if let Some(path) = &self.created_git_path {
            remove_path_best_effort(path);
        }

        for path in self.created_files.iter().rev() {
            remove_path_best_effort(path);
        }

        for path in self.created_dirs.iter().rev() {
            let _ = fs::remove_dir(path);
        }
    }
}

pub(crate) fn is_valid_typed_id(id: &str, prefix: &str) -> bool {
    let Some(uuid) = id.strip_prefix(prefix) else {
        return false;
    };

    !uuid.is_empty() && Uuid::parse_str(uuid).is_ok()
}

#[derive(Debug, Serialize)]
pub struct InitOutput {
    pub store: String,
    pub created: bool,
    pub already_initialized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    pub store: String,
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StatusOutput {
    pub store: String,
    pub valid: bool,
    pub issues: Vec<ValidationIssue>,
    pub counts: StoreCounts,
    pub git: GitSummary,
}

#[derive(Debug, Serialize)]
pub struct StoreCounts {
    pub fragments: usize,
    pub sequences: usize,
    pub captures: usize,
    pub renders: usize,
}

#[derive(Debug, Serialize)]
pub struct GitSummary {
    pub repository: bool,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub dirty: bool,
    pub changed_paths: usize,
    pub untracked_paths: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn resolve_store_path(
    explicit: Option<&Path>,
    alias: Option<&Path>,
) -> Result<PathBuf, AppError> {
    if let Some(path) = explicit.or(alias) {
        return absolutize(path);
    }

    if let Some(path) = env::var_os("PSEQ_STORE").filter(|value| !value.is_empty()) {
        return absolutize(Path::new(&path));
    }

    if let Some(path) = user_config::default_store_path()? {
        return Ok(path);
    }

    let home = user_config::home_dir().ok_or(AppError::DefaultStoreUnavailable)?;

    Ok(home.join(".pseq"))
}

pub fn init_store(path: &Path) -> Result<InitOutput, AppError> {
    let path = absolutize(path)?;
    if path.exists() && !path.is_dir() {
        return Err(AppError::StorePathNotDirectory { path });
    }

    git::require_available()?;

    let preexisting_report = validate_store(&path);
    if preexisting_report.valid {
        return Ok(InitOutput {
            store: paths::display(&path),
            created: false,
            already_initialized: true,
            git_commit: None,
        });
    }

    let target = classify_init_target(&path)?;
    if target == InitTarget::ExistingGitRoot {
        preflight_scaffold_paths(&path)?;
        validate_existing_pseq_content(&path)?;
    }

    let mut scaffold = scaffold_store(&path)?;
    let git_path = path.join(".git");
    let git_path_existed = git_path.exists();

    let git_commit = match init_scaffolded_store(&path, &mut scaffold, git_path_existed) {
        Ok(git_commit) => git_commit,
        Err(error) => {
            scaffold.rollback(&path);
            return Err(error);
        }
    };

    Ok(InitOutput {
        store: paths::display(&path),
        created: true,
        already_initialized: false,
        git_commit,
    })
}

fn init_scaffolded_store(
    path: &Path,
    scaffold: &mut ScaffoldChanges,
    git_path_existed: bool,
) -> Result<Option<String>, AppError> {
    git::ensure_repo(path)?;
    let git_path = path.join(".git");
    if !git_path_existed && git_path.exists() {
        scaffold.created_git_path = Some(git_path);
    }

    let report = validate_store(path);
    if !report.valid {
        return Err(AppError::InitProducedInvalidStore {
            path: path.to_path_buf(),
        });
    }

    git::commit_paths_if_changed(path, &scaffold.touched_paths, "Initialize pseq store")
}

pub fn validate_store(path: &Path) -> ValidationReport {
    validate_store_with_content(path, true)
}

pub(crate) fn require_valid_store_structure(path: &Path) -> Result<(), AppError> {
    let report = validate_store_with_content(path, false);
    if report.valid {
        Ok(())
    } else {
        Err(invalid_store_with_issue_count(path, report.issues.len()))
    }
}

fn classify_init_target(path: &Path) -> Result<InitTarget, AppError> {
    if !path.exists() || is_empty_dir(path)? {
        return Ok(InitTarget::NewOrEmpty);
    }

    if git::worktree_root(path)?
        .as_deref()
        .is_some_and(|root| paths::same_path(root, path))
    {
        Ok(InitTarget::ExistingGitRoot)
    } else {
        Err(AppError::InitTargetNotEmpty {
            path: path.to_path_buf(),
        })
    }
}

fn scaffold_store(path: &Path) -> Result<ScaffoldChanges, AppError> {
    let mut changes = ScaffoldChanges::default();
    if let Err(error) = scaffold_store_inner(path, &mut changes) {
        changes.rollback(path);
        return Err(error);
    }

    Ok(changes)
}

fn scaffold_store_inner(path: &Path, changes: &mut ScaffoldChanges) -> Result<(), AppError> {
    create_dir_all_recording(path, changes)?;
    preflight_scaffold_paths(path)?;

    for dir in REQUIRED_DIRS {
        let dir_path = path.join(dir);
        create_dir_all_recording(&dir_path, changes)?;
        let gitkeep_path = dir_path.join(".gitkeep");
        if write_file_if_missing(&gitkeep_path, "")? {
            changes.created_files.push(gitkeep_path);
            changes
                .touched_paths
                .push(PathBuf::from(dir).join(".gitkeep"));
        }
    }

    let config_path = path.join(config::CONFIG_FILE);
    if !config_path.exists() && write_file_if_missing(&config_path, CONFIG_CONTENT)? {
        changes.created_files.push(config_path);
        changes.touched_paths.push(config::config_pathspec());
    }

    Ok(())
}

fn preflight_scaffold_paths(path: &Path) -> Result<(), AppError> {
    for dir in REQUIRED_DIRS {
        let dir_path = path.join(dir);
        if let Some(metadata) = symlink_metadata(&dir_path)? {
            if metadata.file_type().is_symlink() {
                return Err(init_target_conflict(
                    &dir_path,
                    "required pseq path exists and is a symlink".to_owned(),
                ));
            }
            if !metadata.is_dir() {
                return Err(init_target_conflict(
                    &dir_path,
                    "required pseq path exists and is not a directory".to_owned(),
                ));
            }
        }

        let gitkeep_path = dir_path.join(".gitkeep");
        if let Some(metadata) = symlink_metadata(&gitkeep_path)? {
            if metadata.file_type().is_symlink() {
                return Err(init_target_conflict(
                    &gitkeep_path,
                    "required pseq scaffold path exists and is a symlink".to_owned(),
                ));
            }
            if !metadata.is_file() {
                return Err(init_target_conflict(
                    &gitkeep_path,
                    "required pseq scaffold path exists and is not a file".to_owned(),
                ));
            }
        }
    }

    let config_path = path.join(config::CONFIG_FILE);
    if symlink_metadata(&config_path)?.is_some() {
        validate_existing_config_file(&config_path)?;
    }

    Ok(())
}

fn validate_existing_config_file(path: &Path) -> Result<(), AppError> {
    let Some(metadata) = symlink_metadata(path)? else {
        return Ok(());
    };

    if metadata.file_type().is_symlink() {
        return Err(init_target_conflict(
            path,
            "required pseq config path exists and is a symlink".to_owned(),
        ));
    }

    if !metadata.is_file() {
        return Err(init_target_conflict(
            path,
            "required pseq config path exists and is not a file".to_owned(),
        ));
    }

    let content = fs::read_to_string(path).map_err(|source| AppError::ReadFile {
        path: path.to_path_buf(),
        source,
    })?;
    let _ = config::parse_config(&content).map_err(|message| AppError::InvalidConfig {
        path: path.to_path_buf(),
        message,
    })?;
    Ok(())
}

fn symlink_metadata(path: &Path) -> Result<Option<fs::Metadata>, AppError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(AppError::ReadFile {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn validate_existing_pseq_content(path: &Path) -> Result<(), AppError> {
    let mut issues = Vec::new();
    issues.extend(fragment::validate_fragments(path));
    issues.extend(sequence::validate_sequences(path));
    issues.extend(capture::validate_captures(path));
    issues.extend(render::validate_saved_renders(path));

    if issues.is_empty() {
        Ok(())
    } else {
        Err(init_target_conflict(
            path,
            format!(
                "existing pseq-owned content is invalid ({} issue(s)); run `pseq doctor --store {}`",
                issues.len(),
                paths::display(path)
            ),
        ))
    }
}

fn validate_store_with_content(path: &Path, include_content: bool) -> ValidationReport {
    let path = match absolutize(path) {
        Ok(path) => path,
        Err(error) => {
            let message = error.to_string();
            return ValidationReport {
                store: paths::display(path),
                valid: false,
                issues: vec![validation_issue(
                    "path_resolution_failed",
                    message,
                    Some(path),
                )],
            };
        }
    };

    let mut issues = Vec::new();
    if !path.exists() {
        issues.push(validation_issue(
            "store_missing",
            format!("store directory does not exist: {}", path.display()),
            Some(&path),
        ));
        return report(path, issues);
    }

    if !path.is_dir() {
        issues.push(validation_issue(
            "store_not_directory",
            format!("store path is not a directory: {}", path.display()),
            Some(&path),
        ));
        return report(path, issues);
    }

    let mut content_dirs_safe = true;
    for dir in REQUIRED_DIRS {
        let dir_path = path.join(dir);
        match symlink_metadata(&dir_path) {
            Ok(None) => issues.push(validation_issue(
                "missing_required_dir",
                format!("missing required directory: {}", dir_path.display()),
                Some(&dir_path),
            )),
            Ok(Some(metadata)) if metadata.file_type().is_symlink() => {
                content_dirs_safe = false;
                issues.push(validation_issue(
                    "required_path_symlink",
                    format!(
                        "required path must not be a symlink: {}",
                        dir_path.display()
                    ),
                    Some(&dir_path),
                ));
            }
            Ok(Some(metadata)) if !metadata.is_dir() => {
                issues.push(validation_issue(
                    "required_path_not_directory",
                    format!("required path is not a directory: {}", dir_path.display()),
                    Some(&dir_path),
                ));
            }
            Ok(Some(_)) => {}
            Err(error) => issues.push(validation_issue(
                "required_path_unreadable",
                error.to_string(),
                Some(&dir_path),
            )),
        }
    }

    validate_config(&path, &mut issues);
    validate_git(&path, &mut issues);
    if include_content && content_dirs_safe {
        issues.extend(fragment::validate_fragments(&path));
        issues.extend(sequence::validate_sequences(&path));
        issues.extend(capture::validate_captures(&path));
        issues.extend(render::validate_saved_renders(&path));
    }

    report(path, issues)
}

pub fn require_valid_store(path: &Path) -> Result<(), AppError> {
    let report = validate_store(path);
    if report.valid {
        Ok(())
    } else {
        Err(invalid_store_with_issue_count(path, report.issues.len()))
    }
}

pub(crate) fn invalid_store(path: &Path) -> AppError {
    let report = validate_store(path);
    invalid_store_with_issue_count(path, report.issues.len())
}

pub fn status(path: &Path) -> StatusOutput {
    let report = validate_store(path);
    let path = PathBuf::from(&report.store);
    let git_status = git::status(&path);

    StatusOutput {
        store: report.store,
        valid: report.valid,
        issues: report.issues,
        counts: StoreCounts {
            fragments: collection::files_with_extension(&path, "fragments", "md").len(),
            sequences: collection::files_with_extension(&path, "sequences", "json").len(),
            captures: collection::files_with_extension(&path, "captures", "json").len(),
            renders: collection::files_with_extension(&path, "renders", "md").len(),
        },
        git: GitSummary {
            repository: git_status.error.is_none()
                && git::worktree_root(&path).ok().flatten().is_some(),
            branch: git_status.branch,
            head: git_status.head,
            dirty: git_status.dirty,
            changed_paths: git_status.changed_paths,
            untracked_paths: git_status.untracked_paths,
            error: git_status.error,
        },
    }
}

fn validate_config(path: &Path, issues: &mut Vec<ValidationIssue>) {
    let config_path = path.join(config::CONFIG_FILE);
    let metadata = match symlink_metadata(&config_path) {
        Ok(Some(metadata)) => metadata,
        Ok(None) => {
            issues.push(validation_issue(
                "missing_config",
                format!("missing config file: {}", config_path.display()),
                Some(&config_path),
            ));
            return;
        }
        Err(error) => {
            issues.push(validation_issue(
                "config_unreadable",
                error.to_string(),
                Some(&config_path),
            ));
            return;
        }
    };

    if metadata.file_type().is_symlink() {
        issues.push(validation_issue(
            "config_path_symlink",
            format!(
                "config path must not be a symlink: {}",
                config_path.display()
            ),
            Some(&config_path),
        ));
        return;
    }

    if !metadata.is_file() {
        issues.push(validation_issue(
            "config_not_file",
            format!("config path is not a file: {}", config_path.display()),
            Some(&config_path),
        ));
        return;
    }

    let content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(error) => {
            issues.push(validation_issue(
                "config_not_utf8",
                format!("config file must be UTF-8: {error}"),
                Some(&config_path),
            ));
            return;
        }
    };

    if let Err(message) = config::parse_config(&content) {
        issues.push(validation_issue(
            "config_file_invalid",
            format!("invalid config file: {message}"),
            Some(&config_path),
        ));
    }
}

fn validate_git(path: &Path, issues: &mut Vec<ValidationIssue>) {
    let git_path = path.join(".git");
    if !git_path.exists() {
        issues.push(validation_issue(
            "missing_git_repo",
            format!("missing git repository metadata: {}", git_path.display()),
            Some(&git_path),
        ));
        return;
    }

    match git::worktree_root(path) {
        Ok(Some(root)) => {
            if !paths::same_path(&root, path) {
                issues.push(validation_issue(
                    "git_root_mismatch",
                    format!(
                        "git worktree root is {}; expected {}",
                        root.display(),
                        path.display()
                    ),
                    Some(path),
                ));
            }
        }
        Ok(None) => issues.push(validation_issue(
            "invalid_git_repo",
            format!("path is not a valid git worktree: {}", path.display()),
            Some(path),
        )),
        Err(error) => issues.push(validation_issue(
            "git_unavailable",
            format!("failed to inspect git repository: {error}"),
            Some(path),
        )),
    }
}

fn is_empty_dir(path: &Path) -> Result<bool, AppError> {
    let mut entries = fs::read_dir(path).map_err(|source| AppError::CreateDir {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(entries.next().is_none())
}

fn create_dir_all_recording(path: &Path, changes: &mut ScaffoldChanges) -> Result<(), AppError> {
    for dir in missing_ancestors(path) {
        match fs::create_dir(&dir) {
            Ok(()) => changes.created_dirs.push(dir),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists && dir.is_dir() => {}
            Err(source) => return Err(AppError::CreateDir { path: dir, source }),
        }
    }
    Ok(())
}

fn missing_ancestors(path: &Path) -> Vec<PathBuf> {
    let mut created = Vec::new();
    let mut current = path;

    while !current.exists() {
        created.push(current.to_path_buf());
        let Some(parent) = current.parent() else {
            break;
        };
        current = parent;
    }

    created.reverse();
    created
}

fn write_file_if_missing(path: &Path, content: &str) -> Result<bool, AppError> {
    if let Some(metadata) = symlink_metadata(path)? {
        if metadata.file_type().is_symlink() {
            return Err(init_target_conflict(
                path,
                "required pseq file path exists and is a symlink".to_owned(),
            ));
        }
        if metadata.is_file() {
            return Ok(false);
        }
        return Err(init_target_conflict(
            path,
            "required pseq file path exists and is not a file".to_owned(),
        ));
    }
    fs::write(path, content).map_err(|source| AppError::WriteFile {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(true)
}

fn absolutize(path: &Path) -> Result<PathBuf, AppError> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = env::current_dir().map_err(|source| AppError::CurrentDir { source })?;
        Ok(cwd.join(path))
    }
}

fn report(path: PathBuf, issues: Vec<ValidationIssue>) -> ValidationReport {
    ValidationReport {
        store: paths::display(&path),
        valid: issues.is_empty(),
        issues,
    }
}

fn invalid_store_with_issue_count(path: &Path, issues: usize) -> AppError {
    AppError::InvalidStore {
        path: path.to_path_buf(),
        issues,
    }
}

fn init_target_conflict(path: &Path, message: String) -> AppError {
    AppError::InitTargetConflict {
        path: path.to_path_buf(),
        message,
    }
}

fn remove_path_best_effort(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else {
        let _ = fs::remove_file(path);
    }
}

pub(crate) fn validation_issue(
    code: &str,
    message: String,
    path: Option<&Path>,
) -> ValidationIssue {
    ValidationIssue {
        code: code.to_owned(),
        message,
        path: path.map(paths::display),
    }
}

pub(crate) fn push_duplicate_id_issues(
    issues: &mut Vec<ValidationIssue>,
    ids: BTreeMap<String, Vec<String>>,
    code: &str,
    subject: &str,
) {
    for (id, paths) in ids {
        if paths.len() > 1 {
            issues.push(validation_issue(
                code,
                format!("duplicate {subject} id {id}: {}", paths.join(", ")),
                None,
            ));
        }
    }
}
