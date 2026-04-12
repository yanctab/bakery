#!/bin/bash
#
set -eu

# For now lets install rust in the default
# location which is the home directory
# so it doesn't end up getting in conflict
# with an already existing installation
#export RUSTUP_HOME=/usr/local/rustup
#export CARGO_HOME=/usr/local/cargo
#export PATH=$HOME/.cargo/bin:$PATH
RUST_VERSION=1.82.0

echo "INFO: setup rustup and rust ${RUST_VERSION}"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION
source "$HOME/.cargo/env"
rustup --version
cargo --version
rustc --version

echo "INFO: install cargo-fmt"
rustup component add rustfmt

echo "INFO: setup musl"
rustup target add x86_64-unknown-linux-musl

if [ ! -f /.dockerenv ]; then
   sudo apt -y install musl musl-tools musl-dev build-essential jq
   # We need to install the libssl dev packages to cross-compule openssl for musl
   sudo apt -y install libssl-dev pkg-config
fi
