#[path = "common/mod.rs"]
mod common;

use std::fs;

use common::{
    TestStore, assert_git_clean, assert_stdout_contains, assert_success, issue_codes, path_str,
    pseq, pseq_with_env, pseq_with_stdin, stderr_json, stdout_json,
};

#[path = "sequence_registry/edit.rs"]
mod edit;
#[path = "sequence_registry/lifecycle.rs"]
mod lifecycle;
#[path = "sequence_registry/mutations.rs"]
mod mutations;
#[path = "sequence_registry/validation.rs"]
mod validation;
