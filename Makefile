## Bakery can be built using glibc or musl. Using musl means the binary will be statically
## linked while using glibc it will be dynamically linked. Default is to build using musl.
##
VARIANT ?= musl

export PATH := $(HOME)/.cargo/bin:$(PATH)

## help               - Show this help.
.PHONY: help
help:
	@fgrep -h "##" $(MAKEFILE_LIST) | fgrep -v fgrep | sed -e 's/\\$$//' | sed -e 's/##//'

## build              - Build bakery for x86_64 using musl
.PHONY: build
build: build-musl

## build-glibc        - Build bakery for x86_64 using glibc
.PHONY: build-glibc
build-glibc:
	cargo build

## build-musl         - Build bakery for x86_64 using musl
.PHONY: build-musl
build-musl:
	cargo build --target x86_64-unknown-linux-musl

## build-release      - Build release using glibc or musl, default is musl
.PHONY: build-release
build-release:
	./scripts/do_build_release.sh $(VARIANT)

## format             - Format the code using rustfmt
.PHONY: format
format:
	cargo fmt

## test               - Run tests using cargo
.PHONY: test
test:
	BKRY_PKG_BUILD=test cargo test --locked

## cargo-install      - Install bkry under $HOME/.cargo using cargo
.PHONY: cargo-install
cargo-install:
	cargo install --path --locked .

## install-deb        - Update the current deb bakery package by building a release, create a deb package and install it on the system
.PHONY: install-deb
install-deb: build-release deb-package
	sudo dpkg -i artifacts/bakery.deb

## deb-package        - Create a debian package from the latest release build either using glibc or using musl
.PHONY: deb-package
deb-package: build-release
	./scripts/do_deb_package.sh $(VARIANT)

## inc-version        - Increment minor version
.PHONY: inc-version
inc-version:
	./scripts/do_inc_version.sh

## setup-rust         - Setup rust on local machine supports debian/ubuntu
.PHONY: setup-rust
setup-rust:
	./scripts/setup-rust.sh

## setup-docker       - Setup docker on local machine supports debian/ubuntu
.PHONY: setup-docker
setup-docker:
	./scripts/setup-docker.sh

## docker-shell       - Open a bakery workspace docker shell
docker-shell:
	mkdir -p $(HOME)/.cargo
	mkdir -p $(HOME)/.rustup
	(./docker/do_docker_shell.sh)

## release            - Create a release build, tag and push it to git repo to trigger a release job
.PHONY: release
release: clean inc-version
	./scripts/do_build_release.sh $(VARIANT)
	./scripts/do_deb_package.sh $(VARIANT)
	./scripts/do_release.sh
	git push
	git push --tags

## clean              - Clean
.PHONY: clean
clean:
	cargo clean && rm -r artifacts || true
