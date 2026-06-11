# Introduction

Bakery consists of a number of sub-commands. Each sub-command has it's own flags for more information on what sub-command Bakery supports run

```bash
user@node:/dir$ bkry help
```

For information on each sub-command and what flags are supported run

```bash
user@node:/dir$ bkry <sub-command> -h
```
# Shell

The shell subcommand will start a docker shell and setup the environment for the specified build config.

```bash
user@node:/dir$ bkry shell -c <config>
```

The idea with the Bakery workspace shell is to have an easy environment with direct access to all the tools.
Please see [shell](shell.md) for more information.

# Build

The build sub-command is for starting a build.

```bash
user@node:/dir$ bkry build -c <config>
```

The build config can consist of multiple tasks if no task is specified all that are enabled will be executed. To specify a specific task run

```bash
user@node:/dir$ bkry build -c <config> -t <task>
```

To get a list of what task a build config supports check the build config or run the [List](#List).

# Clean

The clean sub-command is for clean it will currently only remove the build directory created by the build command.

```bash
user@node:/dir$ bkry clean -c <config>
```

# List

The list sub-command will list either all the available build configs in a workspace if non is specified or a list of what tasks a build config supports if a build config is specified

```bash
user@node:/dir$ bkry list -c <config>
```

By default the output is rendered as a human-readable table. Pass `--format json` to get a machine-readable JSON payload suitable for scripting, e.g.:

```bash
user@node:/dir$ bkry list --format json
```

```json
{
  "builds": [
    { "name": "default", "description": "Test Description" }
  ]
}
```

## Context

The list sub-command can also list all the context variables for a specific build config by running

```bash
user@node:/dir$ bkry list -c <config> --ctx
```

This will take the build config and list all the builtin context variables and any one defined in the build config. Can be usefull when setting up the initial workspace or debugging an issue.


# Deploy

The deploy sub-command is a special task with it's own definition in the build config. It is more or less just a proxy for calling a custom deploy script to deploy a build on the target.

```bash
user@node:/dir$ bkry deploy -c <config>
```

For details on how to configure this please see [Deploy](build-config.md#Deploy).

# Upload

The upload sub-command is a special task with it's own definition in the build config. It is more or less just a proxy for calling a custom upload script to upload to an artifact server.

```bash
user@node:/dir$ bkry upload -c <config>
```

For details on how to configure this please see [Upload](build-config.md#Upload)

# Setup

The setup sub-command is a special task with it's own definition in the build config. It is more or less just a proxy for calling a custom setup script to setup the workspace.

```bash
user@node:/dir$ bkry setup -c <config>
```

Currently the setup command is not running inside of docker so any dependency is required to be installed on the host. For details on how to configure this please see [Setup](build-config.md#Setup).

# Sync

The sync sub-command is a special task with it's own definition in the build config. It is more or less just a proxy for calling a custom sync script to sync/update the workspace.

```bash
user@node:/dir$ bkry sync -c <config>
```

Currently the sync command is not running inside of docker so any dependency is required to be installed on the host. For details on how to configure this please see [Sync](build-config.md#Sync).

