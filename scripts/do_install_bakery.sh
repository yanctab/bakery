#!/bin/sh
#

VARIANT=$1

if [ ! -n "${VARIANT}" ]; then
    VARIANT=glibc
fi

check_variant ${VARIANT}

BAKERY_VERSION=$1
TEMP_WORK_DIR=$(mktemp -d --suffix=-bkry-deb)
(cd ${TEMP_WORK_DIR}; wget https://github.com/yanctab/bakery/releases/download/v${BAKERY_VERSION}/bkry-x86_64-${VARIANT}-v${BAKERY_VERSION}.deb)
sudo dpkg -i ${TEMP_WORK_DIR}/bkry-x86_64-${VARIANT}-v${BAKERY_VERSION}.deb
