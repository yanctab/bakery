# Bakery

## Purpose

Bakery is a command-line build engine for Yocto Project / OpenEmbedded
projects. It wraps `bitbake` and runs builds inside Docker to give
developers and CI the same reproducible environment. Project-specific
configuration lives in JSON (build config + workspace config) instead
of ad-hoc shell scripts.

The binary and crate are both named `bkry`. A backward-compatibility
`/usr/bin/bakery` symlink pointing at `bkry` is shipped in the `.deb`
so existing muscle memory keeps working. The tool is written in Rust
and shipped as a statically linked musl binary via `.deb` packages on
GitHub Releases.

## Architecture

- **Language / toolchain:** Rust 2021, default build target
  `x86_64-unknown-linux-musl` (static). A `glibc` variant is also
  supported for local development.
- **Binary:** `src/main.rs` → `bkry`.
- **Packaging:** `.deb` built from `scripts/do_deb_package.sh`. Release
  flow is `scripts/do_build_release.sh` → `do_deb_package.sh` →
  `do_release.sh`, driven by `make release`.
- **CI:** GitHub Actions — `.github/workflows/buildntest.yml` (build &
  test on PRs) and `.github/workflows/release.yml` (tag-triggered
  release job that publishes the `.deb`).
- **Runtime dependency:** Docker. Bakery shells into a workspace
  container image `ghcr.io/yanctab/bakery/bakery-workspace:<version>`
  to run bitbake. Running without Docker is supported via workspace
  config. The GHCR image name keeps the legacy `bakery-workspace`
  slug — only the CLI/crate/.deb are renamed to `bkry`.

## Subcommands

`bkry <command>`:

- `sync` — sync/update git submodules in the workspace
- `setup` — initialise a workspace (git submodules, etc.)
- `build` — run a full build or a single task
- `clean` — clean one or all tasks of a build config
- `list` — list builds or tasks for a build
- `shell` — open a shell inside the Bakery Docker environment
- `deploy` — deploy a built artifact to a target
- `upload` — upload artifacts to an artifactory server

## Configuration model

Two JSON files drive everything:

- **workspace config** — describes workspace layout and defaults
  (`documentation/workspace-config.md`)
- **build config** — per-product config, encapsulates `local.conf` and
  `bblayers.conf` settings and defines tasks
  (`documentation/build-config.md`)

Meta layers are managed by the user (git submodules or `repo`), not
by Bakery.

## Constraints and conventions

- Default builds are musl/static — do not assume a glibc host.
- Tests run with `BKRY_PKG_BUILD=test cargo test --locked`.
- Cargo commands use `--locked` so `Cargo.lock` is authoritative.
- Version bumps go through `scripts/do_inc_version.sh` (`make
  inc-version`), not by hand-editing `Cargo.toml`.
- `BKRY_*` environment variables are the contract between Bakery and
  the bitbake environment; new ones should be documented. The prefix
  stays `BKRY_` regardless of the binary rename.
- The template workspace at `tests/template-workspace` is the
  smoke-test entry point for new contributors.

## Out of scope

- Bakery does not set up meta layers.
- Bakery does not replace bitbake or any Yocto/OE tool — it wraps
  them.
- The `ghcr.io/yanctab/bakery/bakery-workspace` Docker workspace image
  name and the `yanctab/bakery` GitHub repository name are not
  renamed.
