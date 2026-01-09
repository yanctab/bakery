# /etc/bakery/bkry.bashrc sourced in by /etc/bash.bashrc
# when running a bakery-workspace
BKRY_BIN_DIR="/usr/bin"
BKRY_VERSION=$(${BKRY_BIN_DIR}/bakery --version)
BKRY_VERSION=${BKRY_VERSION##* }
BKRY_BRANCH=$(cd ${BKRY_WORK_DIR} && git rev-parse --abbrev-ref HEAD)

update_ps1() {
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    export BKRY_BRANCH=$(git branch --show-current 2>/dev/null)
  fi

  if [ -n "${BKRY_BRANCH}" ]; then
    PS1='\[\033[01;32m\]${BKRY_BUILD_CONFIG}@[${BKRY_BRANCH}]:\[\033[01;34m\]\w\[\033[00m\]\$ '
  fi
}

PROMPT_COMMAND=update_ps1

PATH=${BKRY_BIN_DIR}:${PATH}

# The BKRY_BUILD_CONFIG will be set by
# bakery when initializing a workspace shell
build() {
     (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" build -c "${BKRY_BUILD_CONFIG}" "$@")
}

clean() {
    (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" clean -c "${BKRY_BUILD_CONFIG}" "$@")
}

deploy() {
    (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" deploy -c "${BKRY_BUILD_CONFIG}" "$@")
}

upload() {
    (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" upload -c "${BKRY_BUILD_CONFIG}" "$@")
}

setup() {
    (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" setup -c "${BKRY_BUILD_CONFIG}" "$@")
}

sync() {
    (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" sync -c "${BKRY_BUILD_CONFIG}" "$@")
}

benv() {
    env | grep BKRY
}

help() {
    (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" help "$@")
    echo
    echo -e "\e[1;4mBakery Shell:\e[0m"
    echo
    echo "Inside the bakery shell, a set of helper commands and aliases are available to simplify your workflow."
    echo
    echo -e "\e[1mHelpers:\e[0m These take no arguments and act as simple, useful wrappers."
    echo
    echo -e "\e[1;4mUsage:\e[0m <HELPER>"
    echo
    echo -e "\e[1;4mHelpers:\e[0m"
    echo -e "  \e[1mversion\e[0m  Show the current version of Bakery"
    echo -e "  \e[1mconfig\e[0m   Show the current build config and variant"
    echo -e "  \e[1mbenv\e[0m     Print all Bakery-related env vars (BKRY*)"
    echo -e "  \e[1mctx\e[0m      Print context variables for '${BKRY_BUILD_CONFIG}'"
    echo -e "  \e[1mcdws\e[0m     Change to the workspace directory: '${BKRY_WORK_DIR}'"
    echo -e "  \e[1mbb\e[0m       Run bitbake"
    echo -e "  \e[1meyecandy\e[0m Enable starship as eyecandy"
    echo
    echo -e "\e[1mAliases:\e[0m These wrap Bakery subcommands and default to the current build config."
    echo "They do not require arguments—by default, they pass '-c ${BKRY_BUILD_CONFIG}'."
    echo "To see supported options for any alias, run: <ALIAS> -h"
    echo
    echo -e "\e[1;4mUsage:\e[0m <ALIAS>"
    echo
    echo -e "\e[1;4mAliases:\e[0m"
    echo -e "  \e[1mlist\e[0m     Alias for 'bakery list -c ${BKRY_BUILD_CONFIG}'"
    echo -e "  \e[1mbuild\e[0m    Alias for 'bakery build -c ${BKRY_BUILD_CONFIG}'"
    echo -e "  \e[1mclean\e[0m    Alias for 'bakery clean -c ${BKRY_BUILD_CONFIG}'"
    echo -e "  \e[1msync\e[0m     Alias for 'bakery sync -c ${BKRY_BUILD_CONFIG}'"
    echo -e "  \e[1msetup\e[0m    Alias for 'bakery setup -c ${BKRY_BUILD_CONFIG}'"
    echo -e "  \e[1mdeploy\e[0m   Alias for 'bakery deploy -c ${BKRY_BUILD_CONFIG}'"
    echo -e "  \e[1mupload\e[0m   Alias for 'bakery upload -c ${BKRY_BUILD_CONFIG}'"
    echo
}

list() {
    (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" list -c "${BKRY_BUILD_CONFIG}" "$@")
}

ctx() {
    (cd "${BKRY_WORK_DIR}"; "${BKRY_BIN_DIR}/bakery" list -c "${BKRY_BUILD_CONFIG}" --ctx)
}

version() {
    "${BKRY_BIN_DIR}/bakery" --version
}

config() {
    echo "name: ${BKRY_BUILD_CONFIG}"
}

cdws() {
    cd "${BKRY_WORK_DIR}"
}

bb() {
    (cd "${BKRY_WORK_DIR}"; bitbake "$@")
}

eyecandy() {
    if command -v starship >/dev/null; then
        echo "For best results make sure that the terminal is using Nerd Font"
        echo ""
        echo "    https://www.nerdfonts.com/"
        echo ""
        echo "And make sure that workspace.json contains:"
        echo ""
        echo "   -v ${HOME}/.cache/starship:${HOME}/.cache/starship"
        echo ""
        echo "The actual starship config can be found at"
        echo ""
        echo "   /etc/bakery/bkry-startship.toml"
        echo ""
        echo "The default starship config is based on"
        echo ""
        echo "   https://starship.rs/presets/gruvbox-rainbow"
        echo ""
        echo "For more information please see"
        echo ""
        echo "   "https://github.com/yanctab/bakery/blob/main/documentation/starship.md
        export BKRY_EYECANDY="true"
        bash
    else
        echo "No starship, no eyecandy!"
        echo "Make sure starship https://starship.rs is installed in the docker image"
        exit 1
    fi
}

# Making it possible to have some eyecandy
# 1. Install a Nerd Font for your terminal
# 2. Install in your docker image starship curl -sS https://starship.rs/install.sh | sh
#    if it is based on bakery-workspace then starship is already installed.
# 4. Add the line to workspace.json in the docker.args
#   "-v ${HOME}/.cache/starship:${HOME}/.cache/starship"
if command -v starship >/dev/null; then
    if [ "${BKRY_EYECANDY}" = "true" ]; then
        export STARSHIP_CONFIG=/etc/bakery/bkry-starship.toml
        eval "$(starship init bash)"
    fi
fi
