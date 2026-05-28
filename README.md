# pseq

`pseq` is a simple prompt and command sequencer for CLI agent session management.

## Install

```bash
npm install -g @s-brez/pseq
```

The npm package installs the `pseq` command and includes binaries for:

```text
darwin-arm64
darwin-x64
linux-arm64
linux-x64
win32-x64
```

## Usage

```text
pseq [OPTIONS] <COMMAND>

Commands:
  init       Create a prompt store
  fragment   Manage prompt fragments
  sequence   Manage prompt sequences
  render     Render a sequence
  run        Run sequence turns
  capture    Capture prompt text
  runner     Manage runner commands
  status     Show store status
  diff       Show uncommitted changes
  log        Show store history
  doctor     Check store health
  config     Show configuration

Options:
  -C, --store <PATH>   Use prompt store at PATH
      --json           Print JSON
      --quiet          Suppress success messages
      --no-pager       Disable pager
```

Use `pseq <command> --help` for command-specific help.

## Concepts

`fragment`
: A Markdown prompt file. Fragments may include variables like `{{target}}` or other fragments with `{{pseq.fragment.<fragment-ref>}}`.

`sequence`
: An ordered list of fragments. A sequence renders to one text document. `pseq run` sends each top-level fragment as a separate turn.

`capture`
: Imported prompt text from stdin, a file, or a supported local harness source. Captures can be promoted to fragments and a sequence.

`runner`
: A named command stored in the prompt store config. `pseq` passes prompt text to the command on stdin.


Supported capture sources are `stdin`, `codex`, `claude-code`, `openhands`, and `opencode`.


## Examples 

(todo) 

e.g Run multiple iterations with feedback loop: ...


## JSON

```bash
pseq --json fragment list
pseq --json sequence show Review
pseq --json render Review --var target=src/run.rs
pseq --json run Review --max-captured-output 65536 -- tee /tmp/pseq-turn.txt
```

With `--json`, successful commands write one JSON value to stdout.

Application errors use a JSON error envelope on stderr.

## Files

A prompt store is a Git repository:

```text
fragments/**/*.md
sequences/**/*.json
captures/**/*.json
renders/**/*.md
config.toml
.git/ or .git file
```

Mutating commands commit by default unless `--no-commit` is supplied. Read commands do not create history entries.

Treat stores like source code.

`pseq` does not clone, pull, push, configure remotes, or authenticate to remotes.

