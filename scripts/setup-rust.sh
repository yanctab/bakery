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

dpkgArch="$(dpkg --print-architecture)"
case "${dpkgArch##*-}" in
	amd64) rustArch='x86_64-unknown-linux-gnu'; rustupSha256='0b2f6c8f85a3d02fde2efc0ced4657869d73fccfce59defb4e8d29233116e6db' ;;
	armhf) rustArch='armv7-unknown-linux-gnueabihf'; rustupSha256='f21c44b01678c645d8fbba1e55e4180a01ac5af2d38bcbd14aa665e0d96ed69a' ;;
	arm64) rustArch='aarch64-unknown-linux-gnu'; rustupSha256='673e336c81c65e6b16dcdede33f4cc9ed0f08bde1dbe7a935f113605292dc800' ;;
	i386) rustArch='i686-unknown-linux-gnu'; rustupSha256='e7b0f47557c1afcd86939b118cbcf7fb95a5d1d917bdd355157b63ca00fc4333' ;;
	*) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;;
esac

echo "INFO: setup rustup init"
url="https://static.rust-lang.org/rustup/archive/1.26.0/${rustArch}/rustup-init"
wget "$url"
echo "${rustupSha256} *rustup-init" | sha256sum -c -
chmod +x rustup-init

echo "INFO: setup rust ${RUST_VERSION}"
./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}
rm rustup-init
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
