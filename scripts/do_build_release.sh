#!/bin/sh
#
set -e

VARIANT=$1

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
. $SCRIPT_DIR/lib.sh
VERSION=$(get_bakery_version ${WORK_DIR}/Cargo.toml)

if [ ! -n "${VARIANT}" ]; then
    VARIANT=glibc
fi

check_variant ${VARIANT}

echo "INFO: build bkry v${VERSION} for ${VARIANT}"

if [ "${VARIANT}" = "glibc" ]; then
    (cd ${WORK_DIR}; cargo build --release)
    cp ${WORK_DIR}/target/release/bkry ${ARTIFACTS_DIR}
elif [ "${VARIANT}" = "musl" ]; then
    (cd ${WORK_DIR}; cargo build --target x86_64-unknown-linux-musl --release)
    cp ${WORK_DIR}/target/x86_64-unknown-linux-musl/release/bkry ${ARTIFACTS_DIR}
fi
