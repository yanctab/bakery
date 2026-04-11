#!/bin/sh
#
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
. $SCRIPT_DIR/lib.sh

VERSION=$(get_bakery_version ${WORK_DIR}/Cargo.toml)

echo "INFO: tag bkry v${VERSION}"
git add ${WORK_DIR}/Cargo.toml
git add ${WORK_DIR}/Cargo.lock
git commit -m "v${VERSION}"
git tag v${VERSION}
