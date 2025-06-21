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

## test               - Run all tests using cargo
.PHONY: test
test:
	cargo test

## install            - Install bakery under $HOME/.cargo using cargo
.PHONY: install
install:
	cargo install --path .

## install-deb        - Update the current deb bakery package by building a release, create a deb package and install it on the system
.PHONY: install-deb
install-deb: build-release package
	sudo dpkg -i artifacts/bakery.deb

## package            - Create a debian package from the latest release build either using glibc or using musl
.PHONY: package
package: build-release
	./scripts/do_package.sh $(VARIANT)

## ws-packages            - Create debian packages for the predefined bakery workspaces supported
.PHONY: ws-packages
ws-packages:
	./scripts/do_ws_package.sh --name ea-5.15.32

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
	./scripts/do_package.sh $(VARIANT)
	./scripts/do_release.sh
	git push
	git push --tags

## clean              - Clean
.PHONY: clean
clean:
	cargo clean && rm -r artifacts || true
