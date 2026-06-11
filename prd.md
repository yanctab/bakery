# Add `--format json` to `bkry list` and split data from rendering

## Problem Statement

`bkry list` is the only Bakery subcommand for surfacing what a workspace
contains — supported build configs, the tasks inside a config, and the
context variables a config expands to. Today it is human-only: every
output mode uses hand-written `format!("{:<25} ...")` column padding and
prints directly via `cli.stdout`.

This means:

- CI scripts, workspace bootstrap tooling, and editor integrations
  cannot consume `bkry list` programmatically. The only options are
  parsing fixed-width columns with `awk` (fragile) or re-reading the
  underlying JSON config files (duplicates Bakery's expansion logic for
  `--ctx`).
- The single `execute` method at `src/commands/list.rs:42-105` mixes
  three concerns (mode dispatch, data extraction, output formatting) and
  uses the string `"NA"` as a sentinel for "no `--config` was supplied".
  Adding a new output mode means deeper nesting of an already three-way
  branch.
- Tests at `src/commands/list.rs:232-272` assert on exact stdout strings
  with hardcoded column widths, so any column reshaping is a test
  rewrite even when the underlying data is unchanged.

## Solution

Add a `--format <table|json>` flag to `bkry list`. `table` remains the
default and prints a human-readable view (spacing and headers may be
tweaked from today's exact output); `json` emits a structured, pretty-
printed JSON document on stdout.

Refactor the command internally so the data layer (what to list) is
fully separated from the render layer (how to print it):

- Define view structs in `src/commands/list.rs` that derive
  `serde::Serialize`: `BuildEntry`, `ConfigHeader`, `TaskEntry`, and
  three payload structs `BuildsPayload`, `ConfigTasksPayload`,
  `ConfigContextPayload`.
- `execute` decides which payload to build (based on `--config` and
  `--ctx`), constructs it, then dispatches to one of two free
  functions: `render_table(payload, cli)` or `render_json(payload, cli)`.
- The `"NA"` sentinel goes away; `--config` becomes an `Option<String>`
  in the handler, with `None` meaning "list builds".

The JSON schema uses a header-plus-payload shape so consumers always
know what config they are looking at:

```
$ bkry list --format json
{
  "builds": [
    { "name": "default", "description": "Test Description" }
  ]
}

$ bkry list -c default --format json
{
  "config": {
    "name": "default",
    "arch": "test-arch",
    "machine": "test-machine",
    "description": "Test Description"
  },
  "tasks": [
    { "name": "task1", "description": "NA", "enabled": true },
    { "name": "task2", "description": "test", "enabled": false }
  ]
}

$ bkry list -c default --ctx --format json
{
  "config": {
    "name": "default",
    "arch": "test-arch",
    "machine": "test-machine",
    "description": "Test Description"
  },
  "context": {
    "BKRY_MACHINE": "test-machine",
    "BKRY_ARCH": "test-arch",
    ...
  }
}
```

Errors (e.g. unsupported build config) continue to flow through
`BError` to stderr as plain text with a non-zero exit code, regardless
of `--format`. Stdout in `--format json` mode is therefore either valid
JSON (on success) or empty (on failure) — never a mix.

## User Stories

- As a CI script author, I want `bkry list --format json` so I can pipe
  the supported build configs into `jq` and drive a matrix build,
  instead of brittle column-parsing.
- As a workspace integrator writing an editor extension, I want
  `bkry list -c <config> --format json` so I can populate a task picker
  without re-parsing Bakery's JSON config files or re-implementing
  context expansion.
- As a Bakery developer debugging context expansion, I want
  `bkry list -c <config> --ctx --format json` so I can diff two
  workspaces' resolved context with `diff <(... json) <(... json)`
  instead of squinting at fixed-width tables.
- As an interactive shell user, I want the default
  `bkry list -c <config>` to keep working without flags and still print
  a readable table.
- As a future maintainer adding a new field (say `priority` to tasks),
  I want one place to add it — the view struct — and have both the
  table and JSON renderers pick it up consistently.

## Implementation Decisions

- **Output formats:** `table` (default) and `json`. YAML is not added
  now; `json` already covers the machine-readable case and `serde_yml`
  would add a dependency for no current consumer. Future YAML support
  is a one-line render function once view structs derive `Serialize`.
- **Default format unchanged from user POV (`table`):** Existing
  interactive invocations of `bkry list` keep working with no flag.
  Exact column widths and header text are treated as non-load-bearing
  — tests get updated when the renderer is rewritten, but the human
  experience stays equivalent.
- **JSON shape: header plus payload.** No-config mode returns
  `{"builds":[...]}`; `--config` mode returns `{"config":{...},"tasks":[...]}`
  or `{"config":{...},"context":{...}}`. Consumers feature-detect on
  the top-level keys. Trade-off accepted vs. a discriminated `kind`
  envelope: simpler `jq` expressions outweigh the cost of two
  feature-detect checks.
- **`enabled` field is a boolean (`true`/`false`).** Inverts the
  internal `Task::disabled() -> bool` but reads more naturally in JSON
  and makes `jq 'select(.enabled)'` trivial. The internal data type is
  not renamed.
- **View structs, no `Renderer` trait.** Two render functions
  (`render_table`, `render_json`) live in `src/commands/list.rs` next
  to the view structs. A trait would earn its keep at three or more
  formats; at two, it is ceremony. If YAML is added later, promote to
  a trait then.
- **Schema decoupled from internal data types.** View structs in
  `list.rs` are populated from `workspace.config()` / `workspace.context()`
  but exist independently — `src/data/*.rs` does not gain `Serialize`
  derives. This makes the JSON output a stable public contract
  uncoupled from internal field renames.
- **`"NA"` sentinel removed.** `ListCommand::get_config_name` is
  rewritten to return `Option<String>`; the `default_value("NA")` on
  the clap arg goes away.
- **Errors flow as today.** `BError::CliError` → stderr → non-zero
  exit. No JSON error envelope. The contract is "stdout in
  `--format json` mode is valid JSON xor empty".
- **JSON pretty-printed by default.** `serde_json::to_string_pretty`,
  2-space indent, trailing newline. No `--pretty` flag — `jq` and other
  consumers don't care about whitespace, and humans running the command
  interactively benefit.
- **File layout:** keep `src/commands/list.rs` as one file. Matches
  every other command in `src/commands/`. Estimated post-refactor size
  is under 300 lines plus tests, well within readable bounds.
- **`--format` is scoped to the `list` subcommand.** Not added as a
  global flag. If a future query-shaped subcommand needs it, promote
  then.

## Testing Decisions

- **JSON tests:** in each test, collect every `MockLogger.stdout` call
  into a `String`, join, parse with `serde_json::from_str::<Value>`, and
  assert on structured fields (`v["config"]["name"] == "default"`,
  `v["tasks"][0]["enabled"] == true`). Refactor-friendly: cosmetic
  whitespace or key-order changes do not break tests.
- **Table tests:** keep the existing `MockLogger.expect_stdout(...).with(eq("..."))`
  style. Update the expected strings once for whatever spacing the
  refactor settles on, then leave them as exact matchers.
- **Three test cases per format** (mirroring today's three existing
  tests):
  - no `--config` → `BuildsPayload`
  - `-c default` → `ConfigTasksPayload`
  - `-c default --ctx` → `ConfigContextPayload`
- **Error path:** keep the existing `test_cmd_list_invalid_build_config`
  test for table mode and add a `--format json` variant asserting that
  the returned `BError` matches and that no stdout was emitted (i.e.
  zero `MockLogger.stdout` calls).
- **Schema regression guard:** at minimum one assertion per JSON test
  on every top-level key (`builds`, `config`, `tasks`, `context`) so a
  future accidental rename trips a test.
- **`BKRY_PKG_BUILD=test cargo test --locked`** must pass with the
  refactor in place (project convention; see `CLAUDE.md`).

## Out of Scope

- Adding YAML or any third format. View structs are `Serialize` so the
  door is open, but no consumer is asking for it today.
- Renaming `--ctx` or otherwise reshaping the `list` CLI surface
  beyond adding `--format`. (`list builds` / `list tasks` /
  `list context` as proper subcommands was considered and rejected
  for this PRD.)
- Promoting `--format` to a global flag for other subcommands
  (`build`, `clean`, etc.). Those commands are action-shaped, not
  query-shaped.
- Renaming `Task::disabled() -> bool` in the data layer. The boolean
  inversion lives only in the view struct.
- A JSON error envelope, error-code taxonomy, or any structured
  stderr format.
- Snapshot testing via `insta` or any new dev-dependency.
- Adding `Serialize` derives to `src/data/*.rs` types.
- Splitting `src/commands/list.rs` into a module directory.

## Further Notes

- Existing source: `src/commands/list.rs` (handler + tests, 432 lines
  including tests).
- User-facing docs to update: `documentation/sub-commands.md` (the
  `# List` section starting around line 49 — add a `--format` paragraph
  and a JSON example).
- The `cli.stdout` mock pattern is `src/cli/logger.rs` (`MockLogger`).
- `serde_json` is already a transitive dependency via the config
  layer; no `Cargo.toml` change is expected for the JSON renderer.
  Verify with `cargo tree -i serde_json` during implementation.
- The header object (`config.name`, `config.arch`, `config.machine`,
  `config.description`) is identical between `tasks` and `context`
  modes by design — consumers reading both can share a parser for the
  `config` key.
- Convention reminder from `CLAUDE.md`: tests are run with
  `BKRY_PKG_BUILD=test cargo test --locked`; cargo commands use
  `--locked`.
