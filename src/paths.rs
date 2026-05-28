use std::path::{Path, PathBuf};

pub(crate) fn display(path: &Path) -> String {
    path.display().to_string()
}

pub(crate) fn normalize(path: &Path) -> String {
    path.iter()
        .map(|component| component.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn normalize_reference(reference: &str) -> String {
    reference.replace('\\', "/")
}

pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    left == right
}

pub(crate) fn store_relative(store_path: &Path, path: &Path) -> String {
    path.strip_prefix(store_path)
        .map(normalize)
        .unwrap_or_else(|_| normalize(path))
}

pub(crate) fn next_available_file(
    directory: &Path,
    name: &str,
    fallback: &str,
    extension: &str,
) -> PathBuf {
    let slug = slugify(name, fallback);
    let first = directory.join(format!("{slug}.{extension}"));
    if !first.exists() {
        return first;
    }

    for index in 2.. {
        let candidate = directory.join(format!("{slug}-{index}.{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("unbounded filename search should always return")
}

fn slugify(name: &str, fallback: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash && !slug.is_empty() {
            slug.push('-');
            previous_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        fallback.to_owned()
    } else {
        slug
    }
}
