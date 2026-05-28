use super::*;

#[test]
fn cli_shape_and_aliases_are_public_contract() {
    let top = pseq(&["--help"]);
    assert_success(&top);
    for expected in [
        "Build, render, and run local prompt sequences",
        "Usage: pseq",
        "init",
        "Create a prompt store",
        "doctor",
        "status",
        "log",
        "diff",
        "render",
        "Render a sequence",
        "run",
        "runner",
        "fragment",
        "sequence",
        "capture",
        "config",
        "--store <PATH>",
        "-C, --store <PATH>",
        "--json",
        "--quiet",
        "--no-pager",
    ] {
        assert_stdout_contains(&top, expected);
    }

    let fragment = pseq(&["fragment", "--help"]);
    assert_success(&fragment);
    assert_stdout_contains(&fragment, "Manage prompt fragments");
    for expected in ["new", "list", "show", "edit", "rename", "mv", "rm"] {
        assert_stdout_contains(&fragment, expected);
    }
    let fragment_new = pseq(&["fragment", "new", "--help"]);
    assert_success(&fragment_new);
    for expected in [
        "Create a fragment",
        "Read text from stdin",
        "--dir <PATH>",
        "--path <PATH>",
        "--no-commit",
    ] {
        assert_stdout_contains(&fragment_new, expected);
    }
    let fragment_list = pseq(&["fragment", "list", "--help"]);
    assert_success(&fragment_list);
    for expected in ["--prefix <PATH>", "--tree"] {
        assert_stdout_contains(&fragment_list, expected);
    }

    let sequence = pseq(&["sequence", "--help"]);
    assert_success(&sequence);
    for expected in [
        "new", "list", "show", "edit", "add", "remove", "move", "rename", "mv", "rm",
    ] {
        assert_stdout_contains(&sequence, expected);
    }
    let sequence_new = pseq(&["sequence", "new", "--help"]);
    assert_success(&sequence_new);
    for expected in ["--dir <PATH>", "--path <PATH>"] {
        assert_stdout_contains(&sequence_new, expected);
    }
    let sequence_list = pseq(&["sequence", "list", "--help"]);
    assert_success(&sequence_list);
    for expected in ["--prefix <PATH>", "--tree"] {
        assert_stdout_contains(&sequence_list, expected);
    }

    let capture = pseq(&["capture", "--help"]);
    assert_success(&capture);
    assert_stdout_contains(&capture, "Capture prompt text");
    for expected in [
        "sources", "probe", "last", "range", "import", "list", "show", "mv", "promote",
    ] {
        assert_stdout_contains(&capture, expected);
    }
    let capture_list = pseq(&["capture", "list", "--help"]);
    assert_success(&capture_list);
    assert_stdout_contains(&capture_list, "--prefix <PATH>");

    assert_success(&pseq(&["frag", "--help"]));
    assert_success(&pseq(&["seq", "--help"]));
    assert_success(&pseq(&["cap", "--help"]));

    let runner = pseq(&["runner", "--help"]);
    assert_success(&runner);
    for expected in ["set", "default", "list", "show", "trust", "rm"] {
        assert_stdout_contains(&runner, expected);
    }

    let run = pseq(&["run", "--help"]);
    assert_success(&run);
    for expected in [
        "Run sequence turns",
        "--iterations <N>",
        "--session-scope <SCOPE>",
        "--feedback-from <SOURCE>",
        "--feedback-var <NAME>",
        "--feedback-seed <VALUE>",
    ] {
        assert_stdout_contains(&run, expected);
    }

    let render = pseq(&["render", "--help"]);
    assert_success(&render);
    for expected in [
        "Render a sequence",
        "Set variable; KEY=@FILE reads a file",
        "--dir <PATH>",
        "--path <PATH>",
        "--no-commit",
    ] {
        assert_stdout_contains(&render, expected);
    }
}
