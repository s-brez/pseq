use super::super::*;

pub(super) fn assert_runner_payloads(store: &TestStore) -> TestStore {
    let runner_sink = TestStore::initialized("json-contract-runner-sink");
    let runner_set = pseq(&[
        "runner",
        "set",
        "local",
        "first",
        "--store",
        path_str(store.path()),
        "--json",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(runner_sink.path()),
    ]);
    assert_success(&runner_set);
    assert_object_keys(
        &stdout_json(&runner_set),
        &["name", "slot", "command", "git_commit"],
    );

    let runner_next = pseq(&[
        "runner",
        "set",
        "local",
        "next",
        "--store",
        path_str(store.path()),
        "--json",
        "--",
        pseq_bin(),
        "capture",
        "import",
        "--stdin",
        "--store",
        path_str(runner_sink.path()),
    ]);
    assert_success(&runner_next);

    let runner_default = pseq(&[
        "runner",
        "default",
        "local",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&runner_default);
    assert_object_keys(&stdout_json(&runner_default), &["name", "git_commit"]);

    let runner_list = pseq(&[
        "runner",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&runner_list);
    let runner_list_json = stdout_json(&runner_list);
    assert_object_keys(&runner_list_json, &["runners", "default_runner"]);
    assert_object_keys(
        first_array_item(&runner_list_json, "runners"),
        &["name", "has_next", "is_default"],
    );

    let runner_show = pseq(&[
        "runner",
        "show",
        "local",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&runner_show);
    assert_object_keys(
        &stdout_json(&runner_show),
        &["name", "first", "next", "is_default"],
    );

    let runner_trust = pseq(&[
        "runner",
        "trust",
        "local",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&runner_trust);
    assert_object_keys(&stdout_json(&runner_trust), &["name"]);

    runner_sink
}
