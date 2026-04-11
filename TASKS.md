# Tasks

## Implementation

- [ ] **Verify `cargo install thebakery` installs a working `bakery` binary** [packaging] M
  - Acceptance: `cargo publish --dry-run --locked` succeeds with zero warnings (fix `catagories` typo and the `license`/`license-file` duplication in `Cargo.toml`), `cargo install --path . --locked` from a clean checkout installs a working `bakery` binary whose `bakery --help` output is verified, the `exclude` list in `Cargo.toml` is audited so the published crate still builds (nothing required at build time from `scripts/`, `docker/`, `tests/`, `Makefile` or dotfiles is needed), it is confirmed whether version `1.1.14` already exists on crates.io and bumped via `make inc-version` if needed, a CI job is added (e.g. in `.github/workflows/buildntest.yml`) that runs `cargo publish --dry-run --locked` and `cargo install --path . --locked` on PRs, and `README.md` documents the `cargo install thebakery` install path for end users; renaming the crate/binary is explicitly out of scope.
  - Depends on: none
  - Modify: /mnt/workspace/mans/bakery/Cargo.toml, /mnt/workspace/mans/bakery/README.md, /mnt/workspace/mans/bakery/.github/workflows/buildntest.yml, /mnt/workspace/mans/bakery/.github/workflows/release.yml, /mnt/workspace/mans/bakery/Makefile
  - Create: none
  - Reuse: /mnt/workspace/mans/bakery/Makefile:cargo-install, /mnt/workspace/mans/bakery/Makefile:publish, /mnt/workspace/mans/bakery/src/cli/bakery.rs:env!("CARGO_PKG_VERSION")
  - Risks: `exclude` may drop files needed by a fresh `cargo install` (even though no `build.rs`/`include_str!` were found, `README.md` and `LICENSE` must remain present); `license` + `license-file` conflict may require picking one and updating packaging metadata; version `1.1.14` may already be on crates.io forcing a version bump via `scripts/do_inc_version.sh`; `cargo install` uses the host toolchain (glibc) while the project's default build target is `x86_64-unknown-linux-musl`, so dependencies assumed to be musl-static may behave differently; `cargo publish --dry-run` in CI cannot catch all crates.io server-side rejections.
