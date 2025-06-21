# /etc/bakery/bakery.bashrc sourced in by /etc/bash.bashrc
# when running a bakery-workspace
BKRY_BIN_DIR="/usr/bin"
BKRY_VERSION=$(${BKRY_BIN_DIR}/bakery --version)
BKRY_VERSION=${BKRY_VERSION##* }
BKRY_BRANCH=$(cd ${BKRY_WORK_DIR} && git rev-parse --abbrev-ref HEAD)

# If not running interactively, don't do anything
case $- in
    *i*) ;;
      *) return;;
esac

# don't put duplicate lines or lines starting with space in the history.
# See bash(1) for more options
HISTCONTROL=ignoreboth

# append to the history file, don't overwrite it
shopt -s histappend

# for setting history length see HISTSIZE and HISTFILESIZE in bash(1)
HISTSIZE=1000
HISTFILESIZE=2000

# check the window size after each command and, if necessary,
# update the values of LINES and COLUMNS.
shopt -s checkwinsize

# If set, the pattern "**" used in a pathname expansion context will
# match all files and zero or more directories and subdirectories.
#shopt -s globstar

# make less more friendly for non-text input files, see lesspipe(1)
[ -x /usr/bin/lesspipe ] && eval "$(SHELL=/bin/sh lesspipe)"

# set variable identifying the chroot you work in (used in the prompt below)
if [ -z "${debian_chroot:-}" ] && [ -r /etc/debian_chroot ]; then
    debian_chroot=$(cat /etc/debian_chroot)
fi

# set a fancy prompt (non-color, unless we know we "want" color)
case "$TERM" in
    xterm-color|*-256color) color_prompt=yes;;
esac

# uncomment for a colored prompt, if the terminal has the capability; turned
# off by default to not distract the user: the focus in a terminal window
# should be on the output of commands, not on the prompt
force_color_prompt=yes

if [ -n "$force_color_prompt" ]; then
    if [ -x /usr/bin/tput ] && tput setaf 1 >&/dev/null; then
	# We have color support; assume it's compliant with Ecma-48
	# (ISO/IEC-6429). (Lack of such support is extremely rare, and such
	# a case would tend to support setf rather than setaf.)
	color_prompt=yes
    else
	color_prompt=
    fi
fi

if [ "$color_prompt" = yes ]; then
    PS1='\[\033[01;32m\]${BKRY_BUILD_CONFIG}@[${BKRY_BRANCH}]:\[\033[01;34m\]\w\[\033[00m\]\$ '
else
    PS1='${debian_chroot:+($debian_chroot)}\u@\h:\w\$ '
fi
unset color_prompt force_color_prompt

# If this is an xterm set the title to user@host:dir
case "$TERM" in
xterm*|rxvt*)
    PS1="\[\e]0;${debian_chroot:+($debian_chroot)}\u@\h: \w\a\]$PS1"
    ;;
*)
    ;;
esac

# enable color support of ls and also add handy aliases
if [ -x /usr/bin/dircolors ]; then
    test -r ~/.dircolors && eval "$(dircolors -b ~/.dircolors)" || eval "$(dircolors -b)"
    alias ls='ls --color=auto'
    #alias dir='dir --color=auto'
    #alias vdir='vdir --color=auto'

    alias grep='grep --color=auto'
    alias fgrep='fgrep --color=auto'
    alias egrep='egrep --color=auto'
fi

# colored GCC warnings and errors
#export GCC_COLORS='error=01;31:warning=01;35:note=01;36:caret=01;32:locus=01:quote=01'

# some more ls aliases
alias ll='ls -alF'
alias la='ls -A'
alias l='ls -CF'

# Add an "alert" alias for long running commands.  Use like so:
#   sleep 10; alert
alias alert='notify-send --urgency=low -i "$([ $? = 0 ] && echo terminal || echo error)" "$(history|tail -n1|sed -e '\''s/^\s*[0-9]\+\s*//;s/[;&|]\s*alert$//'\'')"'

# Alias definitions.
# You may want to put all your additions into a separate file like
# ~/.bash_aliases, instead of adding them here directly.
# See /usr/share/doc/bash-doc/examples in the bash-doc package.

if [ -f ~/.bash_aliases ]; then
    . ~/.bash_aliases
fi

# enable programmable completion features (you don't need to enable
# this, if it's already enabled in /etc/bash.bashrc and /etc/profile
# sources /etc/bash.bashrc).
if ! shopt -oq posix; then
  if [ -f /usr/share/bash-completion/bash_completion ]; then
    . /usr/share/bash-completion/bash_completion
  elif [ -f /etc/bash_completion ]; then
    . /etc/bash_completion
  fi
fi

update_ps1() {
  if [ -d ${BKRY_WORK_DIR}/.git ]; then
    if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
        export BKRY_BRANCH=$(git branch --show-current 2>/dev/null)
    fi
  else
    BKRY_BRANCH="NA"
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
