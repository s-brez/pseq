use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::AppError;
use crate::fs_walk;
use crate::paths;
use crate::store::{self, ValidationIssue};

const MANAGED_COLLECTIONS: &[&str] = &["fragments", "sequences", "captures", "renders"];
static CASE_PROBE_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub(crate) struct Destination {
    pub path: PathBuf,
    pub store_relative_path: String,
}

pub(crate) fn files_with_extension(
    store_path: &Path,
    directory: &str,
    extension: &str,
) -> Vec<PathBuf> {
    let root = store_path.join(directory);
    fs_walk::regular_files_with_extensions(&root, &[extension])
}

pub(crate) fn validate_structure(store_path: &Path, directory: &str) -> Vec<ValidationIssue> {
    let root = store_path.join(directory);
    if !root.is_dir() {
        return Vec::new();
    }

    let mut issues = Vec::new();
    collect_structure_issues(&root, &mut issues);
    issues
}

pub(crate) fn collection_relative_path<'a>(
    store_relative_path: &'a str,
    directory: &str,
) -> Option<&'a str> {
    store_relative_path
        .strip_prefix(directory)
        .and_then(|path| path.strip_prefix('/'))
}

pub(crate) fn matches_explicit_path_reference(
    store_relative_path: &str,
    directory: &str,
    extension: &str,
    reference: &str,
) -> bool {
    let normalized = normalize_input(reference);
    let extension = dotted_extension(extension);
    let explicit = normalized == directory
        || normalized.starts_with(&format!("{directory}/"))
        || normalized.contains('/')
        || normalized.ends_with(&extension);

    if !explicit {
        return false;
    }

    let Some(collection_relative) = collection_relative_path(store_relative_path, directory) else {
        return false;
    };
    let reference = strip_collection_prefix(&normalized, directory);
    if reference.is_empty() {
        return false;
    }

    reference == collection_relative
        || (!reference.ends_with(&extension)
            && format!("{reference}{extension}") == collection_relative)
}

pub(crate) fn folded_path_alias(
    store_relative_path: &str,
    directory: &str,
    extension: &str,
) -> Option<String> {
    let collection_relative = collection_relative_path(store_relative_path, directory)?;
    let stem = collection_relative.strip_suffix(&dotted_extension(extension))?;
    if stem.is_empty() {
        return None;
    }

    Some(stem.split('/').collect::<Vec<_>>().join("-"))
}

pub(crate) fn prefix_matches(
    store_relative_path: &str,
    directory: &str,
    extension: &str,
    prefix: Option<&Path>,
) -> Result<bool, AppError> {
    let Some(prefix) = prefix else {
        return Ok(true);
    };
    let Some(collection_relative) = collection_relative_path(store_relative_path, directory) else {
        return Ok(false);
    };

    let normalized = normalize_path(prefix);
    let normalized = strip_collection_prefix(&normalized, directory).trim_end_matches('/');
    if normalized.is_empty() {
        return Ok(true);
    }
    validate_relative_components(normalized, directory, "prefix path")?;

    let extension = dotted_extension(extension);
    let extensionless = collection_relative
        .strip_suffix(&extension)
        .unwrap_or(collection_relative);

    Ok(normalized == collection_relative
        || normalized == extensionless
        || collection_relative.starts_with(&format!("{normalized}/")))
}

pub(crate) fn validate_prefix(directory: &str, prefix: Option<&Path>) -> Result<(), AppError> {
    let Some(prefix) = prefix else {
        return Ok(());
    };
    let normalized = normalize_path(prefix);
    let normalized = strip_collection_prefix(&normalized, directory).trim_end_matches('/');
    if normalized.is_empty() {
        return Ok(());
    }
    validate_relative_components(normalized, directory, "prefix path")
}

pub(crate) fn destination_file(
    store_path: &Path,
    directory: &str,
    extension: &str,
    input: &Path,
) -> Result<Destination, AppError> {
    let normalized = normalize_path(input);
    validate_matching_collection_prefix(&normalized, directory, "destination file")?;
    let relative = strip_collection_prefix(&normalized, directory);
    if relative.is_empty() {
        return Err(invalid_path(
            "destination file",
            &normalized,
            "path must identify a file below the collection",
        ));
    }
    validate_relative_components(relative, directory, "destination file")?;

    let mut components = split_components(relative);
    ensure_target_extension(&mut components, extension, &normalized)?;
    let relative = components.join("/");
    let path = collection_path(store_path, directory, &components);
    preflight_parent_components(store_path, directory, &components)?;
    if symlink_metadata(&path)?.is_some() {
        return Err(invalid_path(
            "destination file",
            &normalized,
            "destination already exists",
        ));
    }

    Ok(Destination {
        path,
        store_relative_path: format!("{directory}/{relative}"),
    })
}

pub(crate) fn destination_directory(
    store_path: &Path,
    directory: &str,
    input: &Path,
) -> Result<PathBuf, AppError> {
    let normalized = normalize_path(input);
    validate_matching_collection_prefix(&normalized, directory, "destination directory")?;
    if normalized.is_empty() {
        return Err(invalid_path(
            "destination directory",
            &normalized,
            "path must not be empty",
        ));
    }
    let mut relative = strip_collection_prefix(&normalized, directory);
    relative = relative.trim_end_matches('/');
    if !relative.is_empty() {
        validate_relative_components(relative, directory, "destination directory")?;
    }

    let components = split_components(relative);
    preflight_directory_components(store_path, directory, &components, &normalized)?;
    Ok(collection_path(store_path, directory, &components))
}

pub(crate) fn default_destination_file(
    store_path: &Path,
    directory: &str,
    name: &str,
    fallback: &str,
    extension: &str,
) -> Destination {
    let path = paths::next_available_file(&store_path.join(directory), name, fallback, extension);
    Destination {
        store_relative_path: paths::store_relative(store_path, &path),
        path,
    }
}

pub(crate) fn destination_file_in_directory(
    store_path: &Path,
    directory: &str,
    extension: &str,
    dir: &Path,
    name: &str,
    fallback: &str,
) -> Result<Destination, AppError> {
    let dir = destination_directory(store_path, directory, dir)?;
    let path = paths::next_available_file(&dir, name, fallback, extension);
    Ok(Destination {
        store_relative_path: paths::store_relative(store_path, &path),
        path,
    })
}

pub(crate) fn create_parent_dir(destination: &Destination) -> Result<(), AppError> {
    if let Some(parent) = destination.path.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

pub(crate) fn same_destination(left: &Path, right: &Path) -> bool {
    let left = comparable_path(left);
    let right = comparable_path(right);
    left == right || same_case_insensitive_destination(&left, &right)
}

fn collect_structure_issues(path: &Path, issues: &mut Vec<ValidationIssue>) {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => {
            issues.push(store::validation_issue(
                "collection_unreadable",
                format!("failed to read managed collection directory: {error}"),
                Some(path),
            ));
            return;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                issues.push(store::validation_issue(
                    "collection_entry_unreadable",
                    format!("failed to read managed collection entry: {error}"),
                    Some(path),
                ));
                continue;
            }
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                issues.push(store::validation_issue(
                    "collection_entry_unreadable",
                    format!("failed to inspect managed collection entry: {error}"),
                    Some(&path),
                ));
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            issues.push(store::validation_issue(
                "collection_entry_symlink",
                format!(
                    "managed collection entries must not be symlinks: {}",
                    path.display()
                ),
                Some(&path),
            ));
        } else if metadata.is_dir() {
            collect_structure_issues(&path, issues);
        } else if !metadata.is_file() {
            issues.push(store::validation_issue(
                "collection_entry_invalid",
                format!(
                    "managed collection entry must be a file or directory: {}",
                    path.display()
                ),
                Some(&path),
            ));
        }
    }
}

fn normalize_path(path: &Path) -> String {
    normalize_input(&path.to_string_lossy())
}

fn normalize_input(value: &str) -> String {
    value.replace('\\', "/")
}

fn strip_collection_prefix<'a>(path: &'a str, directory: &str) -> &'a str {
    if path == directory {
        ""
    } else {
        path.strip_prefix(&format!("{directory}/")).unwrap_or(path)
    }
}

fn validate_relative_components(path: &str, collection: &str, kind: &str) -> Result<(), AppError> {
    if path.is_empty() {
        return Err(invalid_path(kind, path, "path must not be empty"));
    }
    if path.starts_with('/') || has_windows_drive_prefix(path) {
        return Err(invalid_path(kind, path, "absolute paths are not allowed"));
    }

    for component in path.split('/') {
        if component.is_empty() {
            return Err(invalid_path(
                kind,
                path,
                "empty path components are not allowed",
            ));
        }
        if component == "." || component == ".." {
            return Err(invalid_path(
                kind,
                path,
                "parent-directory traversal is not allowed",
            ));
        }
        if component == collection {
            continue;
        }
    }
    Ok(())
}

fn validate_matching_collection_prefix(
    path: &str,
    collection: &str,
    kind: &str,
) -> Result<(), AppError> {
    for managed_collection in MANAGED_COLLECTIONS {
        if managed_collection != &collection
            && (path == *managed_collection || path.starts_with(&format!("{managed_collection}/")))
        {
            return Err(invalid_path(
                kind,
                path,
                "path uses a different managed collection prefix",
            ));
        }
    }
    Ok(())
}

fn split_components(path: &str) -> Vec<String> {
    if path.is_empty() {
        Vec::new()
    } else {
        path.split('/').map(str::to_owned).collect()
    }
}

fn ensure_target_extension(
    components: &mut [String],
    extension: &str,
    original: &str,
) -> Result<(), AppError> {
    let Some(file_name) = components.last_mut() else {
        return Err(invalid_path(
            "destination file",
            original,
            "path must identify a file below the collection",
        ));
    };

    let extension_with_dot = dotted_extension(extension);
    if file_name.ends_with(&extension_with_dot) {
        return Ok(());
    }
    if file_name.rsplit_once('.').is_some() {
        return Err(invalid_path(
            "destination file",
            original,
            &format!("extension must be {extension_with_dot}"),
        ));
    }

    file_name.push_str(&extension_with_dot);
    Ok(())
}

fn preflight_parent_components(
    store_path: &Path,
    directory: &str,
    components: &[String],
) -> Result<(), AppError> {
    let parent_components = components
        .len()
        .checked_sub(1)
        .map(|count| &components[..count])
        .unwrap_or(&[]);
    preflight_directory_components(
        store_path,
        directory,
        parent_components,
        &components.join("/"),
    )
}

fn preflight_directory_components(
    store_path: &Path,
    directory: &str,
    components: &[String],
    original: &str,
) -> Result<(), AppError> {
    let mut current = store_path.join(directory);
    for component in components {
        current.push(component);
        let Some(metadata) = symlink_metadata(&current)? else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            return Err(invalid_path(
                "destination directory",
                original,
                "symlink traversal is not allowed",
            ));
        }
        if !metadata.is_dir() {
            return Err(invalid_path(
                "destination directory",
                original,
                "intermediate path component is not a directory",
            ));
        }
    }
    Ok(())
}

fn collection_path(store_path: &Path, directory: &str, components: &[String]) -> PathBuf {
    let mut path = store_path.join(directory);
    for component in components {
        path.push(component);
    }
    path
}

fn symlink_metadata(path: &Path) -> Result<Option<fs::Metadata>, AppError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(AppError::ReadFile {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn comparable_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };

    if let Ok(path) = absolute.canonicalize() {
        return path;
    }

    if let (Some(parent), Some(file_name)) = (absolute.parent(), absolute.file_name())
        && let Ok(parent) = parent.canonicalize()
    {
        return parent.join(file_name);
    }

    absolute
}

fn same_case_insensitive_destination(left: &Path, right: &Path) -> bool {
    let (Some(left_parent), Some(right_parent), Some(left_name), Some(right_name)) = (
        left.parent(),
        right.parent(),
        left.file_name(),
        right.file_name(),
    ) else {
        return false;
    };

    if left_parent != right_parent {
        return false;
    }

    let left_name = left_name.to_string_lossy();
    let right_name = right_name.to_string_lossy();
    if !left_name.eq_ignore_ascii_case(&right_name) {
        return false;
    }

    directory_is_case_insensitive(left_parent)
}

fn directory_is_case_insensitive(directory: &Path) -> bool {
    if !directory.is_dir() {
        return false;
    }

    let counter = CASE_PROBE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let lower_name = format!(".pseq-case-probe-{}-{counter}", std::process::id());
    let upper_name = lower_name.to_ascii_uppercase();
    if lower_name == upper_name {
        return false;
    }

    let lower_path = directory.join(&lower_name);
    let upper_path = directory.join(&upper_name);
    if lower_path.exists() || upper_path.exists() {
        return false;
    }

    let Ok(file) = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lower_path)
    else {
        return false;
    };
    drop(file);

    let case_insensitive = upper_path.exists();
    let _ = fs::remove_file(&lower_path);
    case_insensitive
}

fn dotted_extension(extension: &str) -> String {
    format!(".{extension}")
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn invalid_path(kind: &str, path: &str, message: &str) -> AppError {
    AppError::InvalidCollectionPath {
        kind: kind.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    }
}
