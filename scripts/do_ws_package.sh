#!/bin/sh
#
set -ex

modify_ws_config()
{
    local _ws_config=$1
    local _ws_name=$2
    local _new_entries='{
    	"layersdir": "sources",
		"configsdir": "/etc/bakery/wss/${_ws_name}",
		"includedir": "/etc/bakery/wss/${_ws_name}/include",
		"scriptsdir": "/opt/bakery/scripts/${_ws_name}",
		"artifactsdir": "artifacts/$#[BKRY_NAME]"
    }'

    jq --argjson new_entries "$_new_entries" '.workspace += $new_entries' "$_ws_config" > tmp.$$.json && mv tmp.$$.json "$_ws_config"
    jq '.mode = "setup"' "$_ws_config" > tmp.$$.json && mv tmp.$$.json "$_ws_config"
}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
. ${SCRIPT_DIR}/lib.sh
. ${SCRIPT_DIR}/shflags

# Define the flags
DEFINE_string 'name' '' 'Specify the name of the workspace.'

# Parse the command-line arguments
FLAGS "$@" || exit 1

# Access the flags
BKRY_WS_NAME="${FLAGS_name}"
BKRY_WS_PKG_NAME="bakery-ws-${BKRY_WS_NAME}"
TEMP_WORK_DIR=$(mktemp -d --suffix=-${BKRY_WS_PKG_NAME})
TEMP_WORK_DIR=${TEMP_WORK_DIR}/${BKRY_WS_PKG_NAME}
VERSION=$(get_bakery_version ${WORK_DIR}/Cargo.toml)
WS_DIR=${WORK_DIR}/workspaces/${BKRY_WS_NAME}
BKRY_CFG_DIR=etc/bakery
BKRY_OPT_DIR=opt/bakery
BKRY_OPT_SCRIPTS_DIR=${BKRY_OPT_DIR}/scripts

#TEMP_WORK_DIR=${WORKSPACE_DIR}/temp
#rm -rf ${TEMP_WORK_DIR} || true

mkdir -p ${TEMP_WORK_DIR}
mkdir -p ${TEMP_WORK_DIR}/${BKRY_CFG_DIR}/wss/${BKRY_WS_NAME}
mkdir -p ${TEMP_WORK_DIR}/${BKRY_OPT_SCRIPTS_DIR}/${BKRY_WS_NAME}
mkdir -p ${TEMP_WORK_DIR}/DEBIAN

cp -a ${WS_DIR}/configs/* ${TEMP_WORK_DIR}/${BKRY_CFG_DIR}/wss/${BKRY_WS_NAME}/
cp -a ${WS_DIR}/scripts/* ${TEMP_WORK_DIR}/${BKRY_OPT_SCRIPTS_DIR}/${BKRY_WS_NAME}
cp -a ${WORK_DIR}/bkry-workspaces/libs ${TEMP_WORK_DIR}/${BKRY_OPT_SCRIPTS_DIR}
cp ${WS_DIR}/workspace.json.in ${TEMP_WORK_DIR}/${BKRY_CFG_DIR}/workspace.json

modify_ws_config ${TEMP_WORK_DIR}/${BKRY_CFG_DIR}/workspace.json ${BKRY_WS_NAME}

touch ${TEMP_WORK_DIR}/DEBIAN/control
cat <<EOT >> ${TEMP_WORK_DIR}/DEBIAN/control
Package: ${BKRY_WS_PKG_NAME}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: all
Maintainer: Mans <mans.zigher@yanct.com>
Description: bakery workspace config supporting ${BKRY_WS_NAME}
EOT

dpkg-deb --root-owner-group --build ${TEMP_WORK_DIR}
cp ${TEMP_WORK_DIR}/../${BKRY_WS_PKG_NAME}.deb ${ARTIFACTS_DIR}/${BKRY_WS_PKG_NAME}-v${VERSION}.deb
(cd ${ARTIFACTS_DIR}; ln -sf ${BKRY_WS_PKG_NAME}-v${VERSION}.deb ${BKRY_WS_PKG_NAME}.deb)
