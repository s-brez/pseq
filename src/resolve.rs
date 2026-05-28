use std::collections::BTreeMap;

use crate::error::AppError;

pub(crate) fn single_match<T>(
    matches: BTreeMap<String, T>,
    not_found: impl FnOnce() -> AppError,
    ambiguous: impl FnOnce(String) -> AppError,
) -> Result<T, AppError> {
    match matches.len() {
        0 => Err(not_found()),
        1 => Ok(matches.into_values().next().expect("one match exists")),
        _ => Err(ambiguous(
            matches.keys().cloned().collect::<Vec<_>>().join(", "),
        )),
    }
}
