#!/bin/sh
set -e
WORKSPACE=$(pwd)
. ${WORKSPACE}/scripts/lib.sh
SETTINGS_FILE=${WORKSPACE}/settings.json
_ARTIFACT_SERVER="https://bakery.jfrog.io/bakery"
_DOCKER_REGISTRY="ghcr.io"
_DOCKER_IMAGE="yanctab/bakery/bakery-workspace"
_DOCKER_DIR=${WORKSPACE}/docker
_DOCKER_TAG=$(get_bakery_version ${WORKSPACE}/Cargo.toml)
_DOCKER_ARGS="--name bakery-workspace \
    --rm \
    -it \
    --dns=8.8.8.8 \
    -v ${WORKSPACE}:${WORKSPACE} \
    -v $HOME/.gitconfig:$HOME/.gitconfig \
    -v $HOME/.ssh:$HOME/.ssh \
    -v $HOME/.bashrc:$HOME/.bashrc \
    -v $HOME/.cargo/bin:$HOME/.cargo/bin \
    -e DOCKER_USER=$USER \
    -e DOCKER_UID=$(id -u) \
    -e DOCKER_GID=$(id -g) \
    -w $(pwd) \
    --entrypoint ${WORKSPACE}/docker/create_docker_user.sh"

get_scripts_dir()
{
  local settings_file=$1
  if [ -f $settings_file ]; then
    echo $(jq -r '.workspace.scriptsdir' < $settings_file)
  else
    echo "${WORKSPACE}"
  fi
}

get_docker_args()
{
  local settings_file=$1
  local docker_args=""

  if [ -f "$settings_file" ]; then
    for arg in $(jq -r '.docker.args[]' < $settings_file); do
      docker_args="$docker_args $arg"
    done
    if [ "x$docker_args" = "x" ]; then
      docker_args=$_DOCKER_ARGS
    fi
  else
    docker_args=$_DOCKER_ARGS
  fi
  eval echo "$docker_args"
}

get_docker_dir()
{
  local settings_file=$1

  if [ -f $settings_file ]; then
    echo $(jq -r '.workspace.dockerdir' < $settings_file)
  else
    echo $_DOCKER_DIR
  fi
}

get_docker_registry()
{
  local settings_file=$1

  if [ -f $settings_file ]; then
    echo $(jq -r '.docker.registry' < $settings_file)
  else
    echo $_DOCKER_REGISTRY
  fi
}

get_docker_image()
{
  local settings_file=$1

  if [ -f $settings_file ]; then
    echo $(jq -r '.docker.image' < $settings_file)
  else
    echo $_DOCKER_IMAGE
  fi
}

get_docker_tag()
{
  local settings_file=$1

  if [ -f $settings_file ]; then
    echo $(jq -r '.docker.tag' < $settings_file)
  else
    if [ -f "${settings_file%/*}/setup.py" ]; then
      version=$(grep version ${settings_file%/*}/setup.py || echo "")
      if [ -n "$version" ]; then
        version=${version#*=}
        version=${version#\"*}
        version=${version%\"*}
        echo $version
      fi
    else
      echo $_DOCKER_TAG
    fi
  fi
}

get_artifact_server()
{

  local settings_file=$1

  if [ -f $settings_file ]; then
    echo $(jq -r '.artifact.server' < $settings_file)
  else
    echo $_ARTIFACT_SERVER
  fi
}

docker_build_image()
{
    local settings_file=$1
    local docker_registry=$2
    local docker_image=$3
    local docker_tag=$4
    local artifact_server=$5
    local docker_dir=$(get_docker_dir $settings_file)

    if [ "$(docker images -q ${docker_registry}/${docker_image}:${docker_tag})" = "" ]; then
      echo "INFO: Start docker build for ${docker_registry}/${docker_image}"
      (docker build -f $DOCKER_DIR/Dockerfile -t ${docker_registry}/${docker_image}:latest --progress=plain .)

      echo "INFO: Tag ${docker_image} version ${docker_tag}"
      (docker tag ${docker_registry}/${docker_image}:latest ${docker_registry}/${docker_image}:${docker_tag})

      #echo "INFO: Push ${docker_image} version ${docker_tag}"
      #(docker push ${docker_registry}/${docker_image}:${docker_tag})
    else
      echo "INFO: ${docker_registry}/$docker_image:$docker_tag already exists no need to build"
    fi
}

docker_inside()
{
    if [ -f /.dockerenv ]; then
        echo "ERROR: it is not supported to run this script inside of docker"
        exit 1
    fi
}

docker_run()
{
  local docker_args="$1"
  shift
  local docker_image="$1"
  shift
  local cmd="$@"

  if [ -f /.dockerenv ]; then
    eval $cmd
  else
    docker run $docker_args $docker_image $cmd || exit $?
  fi
}
