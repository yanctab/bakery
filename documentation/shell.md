# Shell

The shell subcommand will start a docker shell and setup the environment for the specified build config.

```bash
user@node:/dir$ bkry shell -c <config>
```

When starting a Bakery shell the config will be selected and used inside the shell. The terminal will present the following information

```bash
<user>@bakery-v<version>[<config>]:~/$
```

Each subcommand will be available as an alias with the build config predefined. Simply type the sub-command in the shell from any location
no need to specify `bkry` or the build config since it is already preset in the Bakery shell

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

Currently Bakery relies on bash and when the shell is started the /etc/bakery/bkry.bashrc needs to be sourced in by the /etc/bash.bashrc
if the Bakery integration has been done in the docker image used by Bakery. For information on how to to accomplish it please see [custom workspace image](docker.md#custom-worksapce-image).
The /etc/bakery/bkry.bashrc is available at https://github.com/yanctab/bakery/blob/main/scripts/bkry.bashrc.

## Aliases

The Bakery rc-file will extend the shell by defining a couple of Bakery aliases which are just functions making use of the BKRY env variables exposed in the shell.

```bash
Aliases:
  list     Alias for 'bkry list -c product', list all tasks available
  build    Alias for 'bkry build -c product', build all or a specific task
  clean    Alias for 'bkry clean -c product', clean all or a specific task
  sync     Alias for 'bkry sync -c product', sync the workspace
  setup    Alias for 'bkry setup -c product', setup the workspace
  deploy   Alias for 'bkry deploy -c product', deploy firmware to target
  upload   Alias for 'bkry upload -c product', upload firmware to artifactory server
```

## Helpers

The helpers are not Bakery-specific; they can be anything that makes the lives of a developer easier.

```bash
Helpers:
  version  Print version of Bakery
  config   Print current Bakery build config and build variant
  benv     Print all Bakery env variables available starting with BKRY
  ctx      Print all ctx variables available for a 'distro'
```
