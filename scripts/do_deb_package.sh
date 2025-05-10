#!/bin/sh
#
set -e

VARIANT=$1

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
. ${SCRIPT_DIR}/lib.sh
VERSION=$(get_bakery_version ${WORK_DIR}/Cargo.toml)
TEMP_WORK_DIR=$(mktemp -d --suffix=-bakery-deb)

if [ ! -n "${VARIANT}" ]; then
    VARIANT=glibc
fi

check_variant ${VARIANT}

mkdir -p ${TEMP_WORK_DIR}/bakery
TEMP_WORK_DIR=${TEMP_WORK_DIR}/bakery
mkdir -p ${TEMP_WORK_DIR}/usr/bin
mkdir -p ${TEMP_WORK_DIR}/etc/bakery
cp ${ARTIFACTS_DIR}/bakery ${TEMP_WORK_DIR}/usr/bin/
cp ${SCRIPT_DIR}/bkry.bashrc ${TEMP_WORK_DIR}/etc/bakery/bkry.bashrc
cp ${SCRIPT_DIR}/bkry-starship.toml ${TEMP_WORK_DIR}/etc/bakery/bkry-starship.toml

mkdir -p ${TEMP_WORK_DIR}/DEBIAN
touch ${TEMP_WORK_DIR}/DEBIAN/control
cat <<EOT >> ${TEMP_WORK_DIR}/DEBIAN/control
Package: bakery
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: all
Maintainer: Mans <mans.zigher@yanctab.com>
Depends: bash, docker-ce | docker.io
Description: Build engine for the Yocto/OE utilising docker
EOT

cat <<'EOF' > ${TEMP_WORK_DIR}/DEBIAN/postinst
#!/bin/sh

set -e

# Check if Docker is available
if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is not installed. Installing docker.io as fallback..." >&2

  # Update package index and install docker.io
  sudo apt-get update
  sudo apt-get install -y docker.io

  # Check if the installation was successful
  if ! command -v docker >/dev/null 2>&1; then
    echo "Docker installation failed!" >&2
    exit 1
  fi
fi

# Try to determine the user to modify
# Use LOGNAME, USER, or USERNAME — fallback to root
USER_TO_CHECK=${SUDO_USER:-${LOGNAME:-${USER:-root}}}

# Check if the user is in the docker group
if ! id "$USER_TO_CHECK" | grep -qw "docker"; then
  echo "User '$USER_TO_CHECK' is not in the docker group. Adding..."
  sudo usermod -aG docker "$USER_TO_CHECK"
  echo "User '$USER_TO_CHECK' added to the 'docker' group."
  echo "WARNING! You may need to log out and back in for the group change to take effect."
  echo "Please log out and back in (or reboot) for group changes to apply to your shell sessions before trying to run bakery."
fi

echo "postinst completed successfully."
EOF

chmod 755 ${TEMP_WORK_DIR}/DEBIAN/postinst

dpkg-deb --root-owner-group --build ${TEMP_WORK_DIR}

cp ${TEMP_WORK_DIR}/../bakery.deb ${ARTIFACTS_DIR}/bakery-x86_64-${VARIANT}-v${VERSION}.deb
(cd ${ARTIFACTS_DIR}; ln -sf bakery-x86_64-${VARIANT}-v${VERSION}.deb bakery.deb && ln -sf bakery-x86_64-${VARIANT}-v${VERSION}.deb bakery-x86_64-${VARIANT}.deb)

