`pseq` is a simple prompt and command sequencer for light CLI agent automation.

Use `pseq` to:
* configure and run CLI agent loops
* use non-prompt commands in CLI agent loops
* replace or extend written procedures that would otherwise be text prose in a SKILL.md file
* reduce context burden and token usage for known workflows

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

## Examples 

(todo) 
