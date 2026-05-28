#![allow(dead_code)]

use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TestStore {
    path: PathBuf,
}

impl TestStore {
    pub fn new(name: &str) -> Self {
        Self::new_under(&std::env::temp_dir(), name)
    }

    pub fn new_under(base: &Path, name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let path = base.join(format!("pseq-{name}-{}-{nonce}", std::process::id()));
        Self { path }
    }

    pub fn initialized(name: &str) -> Self {
        let store = Self::new(name);
        assert_success(&pseq(&["init", "--store", path_str(store.path())]));
        store
    }

    pub fn initialized_under(base: &Path, name: &str) -> Self {
        let store = Self::new_under(base, name);
        assert_success(&pseq(&["init", "--store", path_str(store.path())]));
        store
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestStore {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn pseq(args: &[&str]) -> Output {
    pseq_command(args).output().expect("pseq binary should run")
}

pub fn pseq_with_env(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = pseq_command(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("pseq binary should run")
}

pub fn pseq_in_dir_with_env(args: &[&str], current_dir: &Path, envs: &[(&str, &str)]) -> Output {
    let mut command = pseq_command(args);
    command.current_dir(current_dir);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("pseq binary should run")
}

pub fn pseq_with_env_changes(args: &[&str], envs: &[(&str, &str)], removed_env: &[&str]) -> Output {
    let mut command = pseq_command(args);
    for key in removed_env {
        command.env_remove(key);
    }
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("pseq binary should run")
}

pub fn pseq_with_env_removed(args: &[&str], removed_env: &[&str]) -> Output {
    let mut command = pseq_command(args);
    for key in removed_env {
        command.env_remove(key);
    }
    command.output().expect("pseq binary should run")
}

pub fn pseq_with_stdin(args: &[&str], stdin: &str) -> Output {
    let mut child = pseq_command(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pseq binary should spawn");

    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(stdin.as_bytes())
        .expect("stdin should be writable");

    child.wait_with_output().expect("pseq binary should finish")
}

fn pseq_command(args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pseq"));
    command.args(args);
    command.env("XDG_CONFIG_HOME", test_config_home(args));
    command
}

fn test_config_home(args: &[&str]) -> PathBuf {
    std::env::current_dir()
        .expect("current dir should resolve")
        .join("target")
        .join("pseq-test-config")
        .join(std::process::id().to_string())
        .join(test_config_key(args))
}

fn test_config_key(args: &[&str]) -> String {
    let key = explicit_store_arg(args).unwrap_or("global");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn explicit_store_arg<'a>(args: &'a [&str]) -> Option<&'a str> {
    let mut index = 0;
    while index < args.len() {
        let arg = args[index];
        if (arg == "--store" || arg == "-C") && index + 1 < args.len() {
            return Some(args[index + 1]);
        }
        if let Some(store) = arg.strip_prefix("--store=") {
            return Some(store);
        }
        index += 1;
    }
    None
}

pub fn pseq_bin() -> &'static str {
    env!("CARGO_BIN_EXE_pseq")
}

pub fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub fn assert_stdout_contains(output: &Output, expected: &str) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(expected),
        "expected stdout to contain {expected:?}, got {stdout:?}"
    );
}

pub fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout should be JSON: {error}\nstdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

pub fn stderr_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stderr).unwrap_or_else(|error| {
        panic!(
            "stderr should be JSON: {error}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

pub fn issue_codes(json: &serde_json::Value) -> Vec<&str> {
    json["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| issue["code"].as_str().unwrap())
        .collect()
}

pub fn assert_git_clean(path: &Path) {
    let output = git(path, &["status", "--porcelain=v1", "--untracked-files=all"]);
    assert_success(&output);
    assert!(
        output.stdout.is_empty(),
        "store should be git-clean, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

pub fn git_head(path: &Path) -> String {
    let output = git(path, &["rev-parse", "HEAD"]);
    assert_success(&output);
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

pub fn git_status(path: &Path) -> String {
    let output = git(path, &["status", "--porcelain=v1", "--untracked-files=all"]);
    assert_success(&output);
    String::from_utf8(output.stdout).unwrap()
}

pub fn git_commit_all(path: &Path, message: &str) {
    assert_success(&git(path, &["add", "--all"]));
    assert_success(&git(
        path,
        &[
            "-c",
            "user.name=pseq-test",
            "-c",
            "user.email=pseq-test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            message,
        ],
    ));
}

pub fn git(path: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("git should run")
}

pub fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path should be valid UTF-8")
}
