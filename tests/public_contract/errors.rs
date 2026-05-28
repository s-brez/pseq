use super::*;

#[test]
fn json_error_envelope_and_exit_codes_are_public_contract() {
    let cli_error = pseq(&["--json", "not-a-command"]);
    assert_eq!(cli_error.status.code(), Some(2));
    assert!(cli_error.stdout.is_empty());
    let cli_error_json = stderr_json(&cli_error);
    assert_object_keys(&cli_error_json, &["error"]);
    assert_object_keys(&cli_error_json["error"], &["code", "message", "details"]);
    assert_eq!(cli_error_json["error"]["code"], "cli_parse_failed");
    assert_eq!(
        cli_error_json["error"]["details"]["kind"],
        "InvalidSubcommand"
    );

    let store = TestStore::new("error-contract");
    fs::create_dir_all(store.path()).unwrap();
    fs::write(store.path().join("unrelated.txt"), "").unwrap();

    let app_error = pseq(&["init", "--store", path_str(store.path()), "--json"]);
    assert_eq!(app_error.status.code(), Some(1));
    assert!(app_error.stdout.is_empty());
    let app_error_json = stderr_json(&app_error);
    assert_object_keys(&app_error_json, &["error"]);
    assert_object_keys(&app_error_json["error"], &["code", "message"]);
    assert_eq!(app_error_json["error"]["code"], "init_target_not_empty");

    let missing = TestStore::new("missing-contract");
    let validation_failure = pseq(&["doctor", "--store", path_str(missing.path()), "--json"]);
    assert_eq!(validation_failure.status.code(), Some(1));
    assert!(validation_failure.stderr.is_empty());
    let validation_json = stdout_json(&validation_failure);
    assert_object_keys(&validation_json, &["store", "valid", "issues"]);
    assert_eq!(validation_json["valid"], false);
    assert_eq!(validation_json["issues"][0]["code"], "store_missing");

    let unsupported_source = pseq(&[
        "capture",
        "probe",
        "--source",
        "unknown-source",
        "--store",
        path_str(missing.path()),
        "--json",
    ]);
    assert_eq!(unsupported_source.status.code(), Some(1));
    assert!(unsupported_source.stdout.is_empty());
    let unsupported_source_json = stderr_json(&unsupported_source);
    assert_eq!(
        unsupported_source_json["error"]["code"],
        "capture_source_unsupported"
    );
}
