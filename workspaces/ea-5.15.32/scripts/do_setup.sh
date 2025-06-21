#!/bin/sh
set -e

SCRIPTS_DIR="$(cd "$(dirname "$0")" && pwd)"
LIBS_DIR="${SCRIPT_DIR}/../libs"

source ${LIBS_DIR}/shflags

# Define the flags
DEFINE_string 'name' '' 'Specify the name of the workspace.'
DEFINE_string 'branch' '' 'Specify the branch to use.'
DEFINE_string 'url' '' 'Specify the repository URL.'
DEFINE_integer 'jobs' 1 'Specify the number of jobs to run.'

# Parse the command-line arguments
FLAGS "$@" || exit 1

# Access the flags
WS_NAME="${FLAGS_name}"
BRANCH="${FLAGS_branch}"
URL="${FLAGS_url}"
JOBS="${FLAGS_jobs}"

repo init -u ${URL} -b ${BRANCH} && repo sync -j${JOBS}