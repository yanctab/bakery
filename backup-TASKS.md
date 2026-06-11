# Tasks

## Implementation

- [x] **Verify cargo install thebakery installs working bakery** [packaging] S
  - Acceptance: `cargo install thebakery` produces a working `bakery` binary; CI validates this in the build-and-test workflow
  - Depends on: none
  - Modify: `.github/workflows/buildntest.yml`, `Cargo.toml`
  - Create: none
  - Reuse: none
  - Risks: none

- [x] **Rename binary, crate, and deb package to bkry (keep Bakery project name)** [packaging] M
  - Acceptance: binary name is `bkry`, crate name is `thebakery`, deb package installs `bkry`; all tests pass
  - Depends on: Verify cargo install thebakery installs working bakery
  - Modify: `Cargo.toml`, `src/main.rs`, `scripts/do_deb_package.sh`, `Makefile`, `.github/workflows/release.yml`
  - Create: none
  - Reuse: none
  - Risks: none

- [ ] **Complete major-release branch -- fix compile errors, warnings, merge main, and verify workspace locking** [packaging] L
  - Acceptance: major-release branch compiles with zero errors and zero new warnings; all tests pass (`make test`); `make lint` is clean; workspace locking metadata feature is functional end-to-end
  - Depends on: Rename binary, crate, and deb package to bkry (keep Bakery project name)
  - Modify: `src/commands/shell.rs`, `src/workspace/settings.rs`, `Cargo.toml`, `src/cli/bakery.rs`, `src/cli/mod.rs`, `src/data/mod.rs`, `src/fs/bitbake.rs`, `src/helper/mod.rs`, `src/workspace/config.rs`, `src/data/artifact.rs`, `src/commands/setup.rs`, `src/commands/mod.rs`
  - Create: none
  - Reuse: `src/workspace/settings.rs:docker_top_dir` (pattern for new `docker_work_dir` getter), `src/commands/mod.rs:BCommand` (trait used in shell.rs tests), `src/constants.rs:BkryConstants` (replaces old `DeejConstants` references)
  - Risks: Merge conflicts from main's bkry rename changes; shell.rs tests reference old "deej" naming throughout; `docker_work_dir()` implementation needs to match the pattern used by `docker_top_dir()` in settings.rs; pre-existing clippy warnings on main (284) are separate from this work; `src/workspace/metadata.rs:213` verify() compares full JSON including `directory` but only reports config/machine/distro/variant mismatches so a workspace directory move is silently ignored; `src/commands/build.rs:165` uses `strip_prefix("BKRY_").unwrap()` which will panic if a non-BKRY_ key enters `extra_ctx`; `src/commands/setup.rs:169` computes `env_variables` via `self.setup_env(env)` but never uses it — `setup.run()` calls `cli.env()` instead, likely a bug
