# Shell

The shell subcommand will start a docker shell and setup the environment for the specified build config.

```bash
user@node:/dir$ bakery shell -c <config>
```

When starting a Bakery shell the config will be selected and used inside the shell. The terminal will present the following information

```bash
<user>@bakery-v<version>[<config>]:~/$
```

Each subcommand will be available as an alias with the build config predefined. Simply type the sub-command in the shell from any location
no need to specify bakery or the build config since it is already preset in the bakery shell

```bash
help
build
list
deploy
upload
setup
sync
```

# Bash

Currently bakery relay on bash and when the shell is started the /etc/bakery/bakery.bashrc needs to be sourced in by the /etc/bash.bashrc
if the bakery integration has been done in the docker image used by bakery. For information on how to to accomplish it please see [custom workspace image](docker.mk# Custom Worksapce Image).
The /etc/bakery.bashrc are available at https://github.com/yanctab/bakery/blob/main/scripts/bakery.bashrc.

## Aliases

The bakery rc-file will extend the shell by defining a couple of bakery alias which is just a functions making use of the BKRY env variables exposed in the shell.

```bash
Aliases:
  list     Aliase for 'bakery list -c product', list all tasks available
  build    Aliase for 'bakery build -c product', build all or a specific task
  clean    Aliase for 'bakery clean -c product', clean all or a specific task
  sync     Aliase for 'bakery sync -c product', sync the workspace
  setup    Aliase for 'bakery setup -c product', setup the workspace
  deploy   Aliase for 'bakery deploy -c product', deploy firmware to target
  upload   Aliase for 'bakery upload -c product', upload firmware to artifactory server
```

## Helpers

The helpers is not bakery specific they can be anthing that can make the lives of a developer easier.

```bash
Helpers:
  version  Print version of bakery
  config   Print current bakery build config and build variant
  benv     Print all bakery env variables available starting with BKRY
  ctx      Print all ctx variables available for a 'distro'
```
