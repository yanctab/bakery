#!/bin/sh
#

VARIANT=$1

if [ ! -n "${VARIANT}" ]; then
    VARIANT=glibc
fi

check_variant ${VARIANT}

BAKERY_VERSION=$1
TEMP_WORK_DIR=$(mktemp -d --suffix=-bakery-deb)
(cd ${TEMP_WORK_DIR}; wget https://github.com/yanctab/bakery/releases/download/v${BAKERY_VERSION}/bakery-x86_64-${VARIANT}-v${BAKERY_VERSION}.deb)
sudo dpkg -i ${TEMP_WORK_DIR}/bakery-x86_64-${VARIANT}-v${BAKERY_VERSION}.deb
