use super::*;

#[test]
fn sequence_create_add_list_and_show_end_to_end() {
    let store = TestStore::initialized("sequences");

    let role = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Role",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "You are a reviewer.\n",
    );
    assert_success(&role);
    let role_json = stdout_json(&role);
    let role_id = role_json["id"].as_str().unwrap();

    let task = pseq_with_stdin(
        &[
            "fragment",
            "new",
            "Task",
            "--stdin",
            "--store",
            path_str(store.path()),
            "--json",
        ],
        "Review this patch.\n",
    );
    assert_success(&task);
    let task_json = stdout_json(&task);
    let task_id = task_json["id"].as_str().unwrap();

    let created = pseq(&[
        "sequence",
        "new",
        "Review Prompt",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&created);
    let created_json = stdout_json(&created);
    let sequence_id = created_json["id"].as_str().unwrap();
    assert!(sequence_id.starts_with("seq_"));
    assert_eq!(created_json["path"], "sequences/review-prompt.json");

    let add_role = pseq(&[
        "sequence",
        "add",
        "Review Prompt",
        "Role",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&add_role);
    let add_role_json = stdout_json(&add_role);
    assert_eq!(add_role_json["fragment"]["id"], role_id);
    assert_eq!(add_role_json["fragment_count"], 1);

    let add_task = pseq(&[
        "seq",
        "add",
        sequence_id,
        "fragments/task.md",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&add_task);
    let add_task_json = stdout_json(&add_task);
    assert_eq!(add_task_json["fragment"]["id"], task_id);
    assert_eq!(add_task_json["fragment_count"], 2);

    let list = pseq(&[
        "sequence",
        "list",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&list);
    let list_json = stdout_json(&list);
    assert_eq!(list_json["sequences"].as_array().unwrap().len(), 1);
    assert_eq!(list_json["sequences"][0]["id"], sequence_id);
    assert_eq!(list_json["sequences"][0]["fragment_count"], 2);

    let show = pseq(&[
        "sequence",
        "show",
        "Review Prompt",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    let show_json = stdout_json(&show);
    assert_eq!(show_json["id"], sequence_id);
    assert_eq!(show_json["fragments"][0]["id"], role_id);
    assert_eq!(show_json["fragments"][0]["name"], "Role");
    assert_eq!(show_json["fragments"][1]["id"], task_id);
    assert_eq!(show_json["fragments"][1]["name"], "Task");
    assert!(show_json["fragments"][0].get("body").is_none());

    let show_by_path = pseq(&[
        "sequence",
        "show",
        "sequences/review-prompt.json",
        "--store",
        path_str(store.path()),
    ]);
    assert_success(&show_by_path);
    assert_stdout_contains(&show_by_path, "Review Prompt");
    assert_stdout_contains(&show_by_path, "1.");
    assert_stdout_contains(&show_by_path, "Role");
    assert_stdout_contains(&show_by_path, "2.");
    assert_stdout_contains(&show_by_path, "Task");

    let sequence_file = fs::read_to_string(store.path().join("sequences/review-prompt.json"))
        .expect("sequence file should be readable");
    let sequence_file_json: serde_json::Value = serde_json::from_str(&sequence_file).unwrap();
    assert_eq!(sequence_file_json["fragments"][0], role_id);
    assert_eq!(sequence_file_json["fragments"][1], task_id);
    assert!(
        !sequence_file_json
            .to_string()
            .contains("You are a reviewer")
    );

    assert_git_clean(store.path());
}

#[test]
fn sequence_add_can_insert_fragment_at_one_based_index() {
    let store = TestStore::initialized("sequence-insert");

    for (name, body) in [
        ("Intro", "intro\n"),
        ("Body", "body\n"),
        ("Outro", "outro\n"),
    ] {
        assert_success(&pseq_with_stdin(
            &[
                "fragment",
                "new",
                name,
                "--stdin",
                "--store",
                path_str(store.path()),
            ],
            body,
        ));
    }

    assert_success(&pseq(&[
        "sequence",
        "new",
        "Inserted",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Inserted",
        "Intro",
        "--store",
        path_str(store.path()),
    ]));
    assert_success(&pseq(&[
        "sequence",
        "add",
        "Inserted",
        "Outro",
        "--store",
        path_str(store.path()),
    ]));

    let inserted = pseq(&[
        "sequence",
        "add",
        "Inserted",
        "Body",
        "--at",
        "2",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&inserted);
    let inserted_json = stdout_json(&inserted);
    assert_eq!(inserted_json["index"], 2);
    assert_eq!(inserted_json["fragment_count"], 3);

    let show = pseq(&[
        "sequence",
        "show",
        "Inserted",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_success(&show);
    let show_json = stdout_json(&show);
    assert_eq!(show_json["fragments"][0]["name"], "Intro");
    assert_eq!(show_json["fragments"][1]["name"], "Body");
    assert_eq!(show_json["fragments"][2]["name"], "Outro");

    let invalid_insert = pseq(&[
        "sequence",
        "add",
        "Inserted",
        "Body",
        "--at",
        "5",
        "--store",
        path_str(store.path()),
        "--json",
    ]);
    assert_eq!(invalid_insert.status.code(), Some(1));
    assert!(invalid_insert.stdout.is_empty());
    assert_eq!(
        stderr_json(&invalid_insert)["error"]["code"],
        "invalid_sequence_index"
    );

    let rendered = pseq(&["render", "Inserted", "--store", path_str(store.path())]);
    assert_success(&rendered);
    assert_eq!(
        String::from_utf8(rendered.stdout).unwrap(),
        "intro\nbody\noutro\n"
    );
    assert_git_clean(store.path());
}
