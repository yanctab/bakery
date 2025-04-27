# Introduction

The build config is what describes building the product for bakery. A typical build config would have a similare format as bellow


```json
{
        "version": "6",
        "name": "",
        "product": "",
        "project": "",
        "description": "",
        "arch": "",
        "context": [
        ],
        "bb": {
                "machine": "",
                "distro": "",
                "initenv": "",
                "localconf": [
                ],
                "bblayersconf": [
                ]
        },
        "include": [
        ],
        "tasks": {
                "task1": {
                        "index": "0",
                        "name": "task1",
                        "disabled": "false",
                        "condition": "true",
                        "recipes": [],
                        "artifacts": []
                },
                "task2": {
                        "index": "1",
                        "type": "non-bitbake",
                        "disabled": "false",
                        "condition": "true",
                        "name": "task2",
                        "builddir": "",
                        "build": "",
                        "clean": "",
                        "artifacts": []
                }
        },
        "deploy": {
                "cmd": ""
        },
        "upload": {
                "cmd": ""
        },
        "setup": {
                "cmd": ""
        },
        "sync": {
                "cmd": ""
        }
}
```

The build config can be split up in

* Config data   - configuration data specific for config like config file format version and config name which is currently the same as the product name.
* Product data  - product data like name, description, arch.
* Context data  - context variables that can be used throught the build config.
* Bitbake data  - bitbake data distro, machine, deploy dir, init-env script, local.conf and bblayers.conf.
* Tasks data    - to build a product multiple tasks might be required. The tasks data contains a list of tasks and defines what each task should do.
  * Artifacts data - The artifacts data is part of the task data and contains what artifacts to collect for each task.
* Deploy data   - information on how to deploy an image to the target.
* Upload data   - information on how to upload firmware to a artifactory server.
* Setup data    - information on how to setup the workspace e.g. initialization of git submodules
* Sync data     - information on how to sync the workspace e.g. sync/update of git submodules

# Config Data

## version

The config version is used to mark breaking changes to the build config format currently it is at version 5. If the format changes the version will be bumped and bakery will allert that the build config format needs to be migrated to the new format.

## name, product & project

Normaly only the name is needed and the product and project will be set to match the name but sometimes the product and projects needs to be different and can then be set to a specific value that is different from the name of the build config.

# Product Data

## name

The same as the build config name currently.

## description

A short description of the product listed when running the [List](sub-commands.md#List) sub-command.

## arch

What arch the product is. Might not be used but can be used as a context variable in paths or scripts. It is normally part of the naming convention of the artifacts after built an image in bitbake.

# Context Data

The context data is a list of variables that can be used by the rest of the build config. To use a context variable in the build config declare it in the context list in the build config

```json
{
  "context": [
    "CTX_VAR=test"
  ]
}
```

This context variable can then be used in the build config by wrapping it inside

```json
{
  "bb": {
    "localconf": [
      "$#[CTX_VAR]"
    ]
}
```

Any context variable in the build config will be expanded when bakery is parsing the build config. The context is a concept that is made up of two context variable type "built-in" variables and "config" variables. The "config" context variables are the once defined in the context section of the build config while the "built-in" variables are comming from the bakery binary. The values of the "built-in" variables are either defined by the workspace.json or by a combination that the bakery binary will define in run-time. Currently the following "built-in" variables are avilable to be used in the build config

```
BKRY_MACHINE
BKRY_ARCH
BKRY_DISTRO
BKRY_BB_BUILD_DIR
BKRY_BB_DEPLOY_DIR
BKRY_PRODUCT_NAME
BKRY_ARTIFACTS_DIR
BKRY_LAYERS_DIR
BKRY_SCRIPTS_DIR
BKRY_BUILDS_DIR
BKRY_WORK_DIR
BKRY_PLATFORM_VERSION
BKRY_BUILD_ID
BKRY_PLATFORM_RELEASE
BKRY_BUILD_SHA
BKRY_BUILD_VARIANT
BKRY_RELEASE_BUILD
BKRY_ARCHIVER
BKRY_DEBUG_SYMBOLS
BKRY_DEVICE
BKRY_IMAGE
BKRY_DATE
BKRY_TIME
```

To get the up to date list please refere to [BUILT_IN_CONTEXT_VARIABLES](https://github.com/yanctab/bakery/blob/main/src/data/context.rs#L13). Some of the "built-in" context variables will be exposed to the bitbake environment by getting included to the local.conf. To get a list of what context variables a build config offeres and the values of them run the [list](sub-commands.md#context) sub-command with --ctx flag.

## BKRY_DATE and BKRY_TIME

The BKRY_DATE and BKRY_TIME context variables will be expanded to the current date and time. Currently the format is hardcoded to YY-MM-DD and HH:MM but shortly locale should be used so the format is picked up from the system instead.

# Bitbake Data

The bitbake data in the build config is the data required when calling the bitbake command to build the image.

```json
        "bb": {
                "machine": "",
                "distro": "",
                "deploydir": "",
                "initenv": "",
                "localconf": [
                ],
                "bblayersconf": [
                ]
        },
```

## distro

The distro value is mapped to the bitbake [DISTRO](https://docs.yoctoproject.org/singleindex.html#term-DISTRO) variable. This will be set in the local.conf file generated by bakery in the builds directory used by bitbake.

### Context

The value of the distro will be added to the built in context variable 

```
BKRY_DISTRO
```

## machine

The machine value is mapped to the bitbake [MACHINE](https://docs.yoctoproject.org/singleindex.html#term-DISTRO) variable. This will be set in the local.conf file generated by the bakery in the build directory used by bitbake.

### Context

The value of the machine will be added to the built in context variable

```
BKRY_MACHINE
```

## initenv

Before running the bitbake command the bitbake environment needs to be setup this is done by sourcing a file

```bash
source layers/poky/oe-init-build-env <build dir>
```

Bakery will source this file to setup the environment both when executing the [build](sub-commands.mk#Build) and when setting up a [shell](sub-commands.mk#Shell).


## localconf

Bakery will take the localconf data and combine it with the other bitbake data available and create a local.conf in the builds dir defined in the [workspace conf](workspace-config.md#Customize). The local.conf is then used by bitbake when building the image. A [dry-run](sub-commands.mk#Dry-Run) can be executed using bakery to generate the bitbake configurations files and skipp the build.


## bblayersconf

Bakery will take the bblayersconf data and generate the bblayers.conf in the builds dir defined in the [workspace conf](workspace-config.md#Customize). The bblayers.conf is used by bitbake to parse meta layers before starting the build.

# Include Configs

There are cases where multiple product build configs are defined in a workspace where these product are using the same tasks and/or the custome sub-commands. Each product could have it's own specific context variables that the tasks and custome sub-commands. This will prevent duplication of build data in the build configs. To include config add to the build config

```json
        "include": [
          "tasks"
          "subcommands"
        ],
```

Bakery will by default look for tasks.json and subcommands.json under includes dir in the configs dir defined in the workspace.json. If nothing is define in the workspace.json it will search for the included build configs under

```bash
configs/include
```

Both the configs dir and include dir can be set in the workspace.json for more information please see [workspace config](workspace-config.md). The format of a included build config is the same as the product build config but it should only include the defined tasks and custom subcommands.

# Tasks Data

The tasks data contains a list of tasks needed to build a product.

```json
        "tasks": {
        },
```

## index

This is just to make sure that the tasks are executed in an expected order will most likely be removed in a later release.

## name

The name of the task should be unique

## type

A task can be a bitbake task or a non-bitbake task. The different types looks has a slightly different format. Default value is bitbake.

## disabled

Sometimes a task is needed but it should not be executed by default when not specifing a task and running a full build. For example a signing task that requires some additional resources like an HSM when signing so it should only be executed by a specific signing node then it can be disabled. It will then only be executed when the task is specificelly specified in the bakery command using the the task flag in the [build](sub-commands.md#Build).

## condition

Sometimes a task needs to only run under a specific condition. By default the condition is true but it is possible to use a [Context](build-config.md#context). For example bakery has the variant flag which will set the context variable $#[BKRY_RELEASE_BUILD] to one which can then be used as a condition to only execute a specific task.

```json
{
  "task": {
    "index": "0",
    "name": "task",
    "condition": "$#[BKRY_RELEASE_BUILD]",
    "recipes": [
      "recipe"
    ],
    "artifacts": []
  }
}
```

### Custom Condition

To define a custom condition define a context variable in the [context data](build-config.md#Context Data).

```json
{
  "context": [
    "MY_CONDITION=0"
  ]
}
```

Define a condition task

```json
{
  "task": {
    "index": "0",
    "name": "task",
    "condition": "$#[MY_CONDITION]",
    "recipes": [
      "recipe"
    ],
    "artifacts": []
  }
}
```

The conditional task will only run if condition is 1. The conditional task can now be triggered when running the build command
using the context flag --ctx that can be used to set any of the context variables.

```
build --ctx MY_CONDITION=1
```

This can be usefull when the CI might have some special tasks or if a task only should run during a --variant release build.

### bitbake

```json
{
  "task1": {
    "index": "0",
    "type": "bitbake",
    "disabled": "false",
    "name": "",
    "recipes": [],
    "artifacts": []
  }
}
```

#### recipes

The recipes is only used if the task is of type "bitbake". A list of recipes that the task should execute. Normally a task contains one recipe and it is normally a image recipe but any recipe can be called.

```json
{
  "recipes": [
    "core-image-minimal"
  ]
}
```

##### task

A recipe task can also be defined so sometimes when building an image and an sdk is needed then that can be defined accordingly

```json
{
  "recipes": [
    "core-image-minimal",
    "core-image-minimal:do_populate_sdk"
  ]
}
```

Any recipe task can be defined.

### non-bitbake


```json
{
  "task2": {
    "index": "0",
    "type": "non-bitbake",
    "disabled": "false",
    "name": "",
    "builddir": "",
    "build": "",
    "clean": "",
    "artifacts": []
  }
}
```

#### build

Sometimes there are taskes needed to be executed after the image has been built. One such example is signing of firmware. Then a non-bitbake task can be defined.


```json
{
  "sign-image": {
    "index": "0",
    "type": "non-bitbake",
    "name": "sign-image",
    "disabled": "true",
    "builddir": "$#[SIGNING_DIR]",
    "build": "$#[BKRY_SCRIPTS_DIR]/sign.sh $#[BKRY_IMAGE]",
    "clean": "",
    "artifacts": []
  }
}
```

#### clean

The clean command is only used by the non-bitbake task it can be a shell command or a script that is called.


```json
{
  "sign-image": {
    "index": "0",
    "type": "non-bitbake",
    "name": "sign-image",
    "disabled": "true",
    "builddir": "$#[SIGNING_DIR]",
    "build": "$#[BKRY_SCRIPTS_DIR]/sign.sh $#[BKRY_IMAGE]",
    "clean": "$#[BKRY_SCRIPTS_DIR]/clean.sh $#[BKRY_IMAGE]",
    "artifacts": []
  }
}
```

#### builddir

The builddir is only used by the non-bitbake task and is used to change working directory before executing the build or clean command.

## artifacts

Each task has the capability to collect specific files. All collected files will be placed in the artifacts directory, which is defined in the workspace config. The artifacts directory is specified by the context variable BKRY_ARTIFACTS_DIR. I will refer to the artifacts directory using the context variable BKRY_ARTIFACTS_DIR.

Artifacts are organized as a list of children, where each child can have a type. If no type is specified, the default type "file" will be used.

### file

Collect file 'test/file1.txt' and copy it to 'BKRY_ARTIFACTS_DIR/file1.txt'.

```json
  "artifacts": [
        {
            "source": "test/file1.txt"
        },
        {
            "source": "test/file2.txt",
            "dest": "test/renamed-file2.txt"
        }
  ]
```

Rename 'test/file2.txt' to 'renamed-file2.txt' and copy it 'BKRY_ARTIFACTS_DIR/test/'.


### directory

Create a directory in the 'BKRY_ARTIFACTS_DIR' directory named 'dir' and copy all artifacts under 'BKRY_ARTIFACTS_DIR/dir/'

```json
  "artifacts": [
      {
          "type": "directory",
          "name": "dir",
          "artifacts": [
              {
                  "source": "file1.txt"
              },
              {
                  "source": "file2.txt",
                  "dest": "renamed-file2.txt"
              }
          ]
      }
  ]
```

### archive

Create a archive in the 'BKRY_ARTIFACTS_DIR' directory named 'test.zip' and collect the all artifacts in the archive

```json
  "artifacts": [
          "type": "archive",
          "name": "test.zip",
          "artifacts": [
              {
                  "source": "file1.txt",
                  "dest": "renamed-file2.txt"
              }
          ]
  ]
```

The archive type currently supports the following archives zip, tar.bz2 and tar.gz.

### manifest

Create a manifest file in the 'BKRY_ARTIFACTS_DIR' directory named 'test-manifest.json'. The manifest can contain build data.

```json
  "artifacts": [
        {
            "type": "manifest",
            "name": "test-manifest.json",
            "content": {
                "machine": "$#[BKRY_MACHINE]",
                "date": "$#[BKRY_DATE]",
                "time": "$#[BKRY_TIME]",
                "arch": "$#[BKRY_ARCH]",
                "distro": "$#[BKRY_DISTRO]",
                "sha": "$#[BKRY_BUILD_ID]",
                "variant": "$#[BKRY_BUILD_VARIANT]",
                "version": "$#[BKRY_PLATFORM_VERSION]"
            }
        }
  ]
```

### link

Create a symbolic link in the 'BKRY_ARTIFACTS_DIR' directory named 'link.txt' pointing to 'test/file.txt'.

```json
  "artifacts": [
        {
            "type": "link",
            "name": "link.txt",
            "source": "test/file.txt"
        }
  ]
```

### conditional

Create a symbolic link in the 'BKRY_ARTIFACTS_DIR' directory named 'link.txt' pointing to 'test/file.txt' if the 'condition' is true.

```json
  "artifacts": [
        {
            "type": "conditional",
            "condition": "$#[BKRY_ARCHIVER]",
            "artifacts": [
              {
                "type": "link",
                "name": "link.txt",
                "source": "test/file.txt"
              }
            ]
        }
  ]
```

The following conditions are interpreted as true

```bash
"1" | "yes" | "y" | "Y" | "true" | "YES" | "TRUE" | "True" | "Yes"
```

### Context

All context variables can be used in the artifacts the only place where context variables cannot be used is in the 'type' for the artifacts.

# Custom Sub-Commands

The custom sub-commands are to define sub-commands that is acting more like proxies so that bakery can be used as one tool for the entire work-flow when building, cleaning, deploying, uploading, setup and syncing. The custom sub-commands are likely the same for most products so it is recommended to use the context variables for product specific data and then use the context variables when calling the defined custom sub-command. The sub-commands can call either a script or a specific command. Each custom sub-command is also exposed in the bakery workspace shell for easy access.

## deploy

The deploy section currently is just made up of a cmd. This can be used to define a custom deploy command making use of the context variables. If not default a default echo command will be used

```json
"deploy": {
        "cmd": "$#[BKRY_SCRIPTS_DIR]/deploy.sh $#[BKRY_ARTIFACTS_DIR]/full-image-$#[BKRY_MACHINE].mender $#[BKRY_DEVICE]"
}
```

## upload

The upload section currently is just made up of a cmd. This can be used to define a custom upload command making use of the context variables.If not default a default echo command will be used

```json
"upload": {
        "cmd": "$#[BKRY_SCRIPTS_DIR]/upload.sh $#[BKRY_ARTIFACTS_DIR]/full-image-$#[BKRY_MACHINE].mender $#[MENDER_ARTIFACT_SERVER]"
}
```

## setup

The setup section currently is just made up of a cmd. This can be used to define a custom setup command making use of the context variables.If not default a default echo command will be used

```json
"setup": {
        "cmd": "$#[BKRY_SCRIPTS_DIR]/setup.sh"
}
```

## sync

The sync section currently is just made up of a cmd. This can be used to define a custom sync command making use of the context variables.If not default a default echo command will be used

```json
"sync": {
        "cmd": "$#[BKRY_SCRIPTS_DIR]/sync.sh"
}
```

# Bitbake

## local.conf

The following context variables will also be exposed as bitbake variables in the local.conf file

```
BKRY_MACHINE
BKRY_DISTRO
BKRY_PRODUCT_NAME
BKRY_PLATFORM_VERSION
BKRY_BUILD_ID
BKRY_BUILD_SHA
BKRY_RELEASE_BUILD
BKRY_BUILD_VARIANT
BKRY_PLATFORM_RELEASE
```

They will get the prefix BKRY stripped appended to the end of the local.conf. Run a build using the --dry-run flag
to produce the local.conf and bblayers.conf to view the genereated files under the build dir something like bellow
should be presented in tmp/conf/local.conf.

```
MACHINE ?= "imx8mm-var-dart"
PRODUCT_NAME ?= "boss"
DISTRO ?= "boss"
SSTATE_DIR ?= "/mnt/workspace/bkry-workspace/.cache/imx8mm/sstate-cache"
DL_DIR ?= "/mnt/workspace/bkry-workspace/.cache/download"
PLATFORM_VERSION ?= "0.0.0"
BUILD_ID ?= "0"
BUILD_SHA ?= "dev"
RELEASE_BUILD ?= "0"
BUILD_VARIANT ?= "dev"
PLATFORM_RELEASE ?= "0.0.0-0"
```

The exakt content depends on what has been put in the build config in the [bitbake data](build-config.md#Bitbake Data)
section. The bitbake data together with the context variables

```
BKRY_MACHINE
BKRY_DISTRO
BKRY_PRODUCT_NAME
BKRY_PLATFORM_VERSION
BKRY_BUILD_ID
BKRY_BUILD_SHA
BKRY_RELEASE_BUILD
BKRY_BUILD_VARIANT
BKRY_PLATFORM_RELEASE
```

The BKRY_MACHINE, BKRY_DISTRO and BKRY_PRODUCT_NAME are defined by the bitbake data plus the name field while the

```
BKRY_PLATFORM_VERSION
BKRY_BUILD_ID
BKRY_BUILD_SHA
BKRY_RELEASE_BUILD
BKRY_BUILD_VARIANT
BKRY_PLATFORM_RELEASE
```

can all be set either by assigning them in the [context data](build-config.md#Context Data) section of the build config or by assigning them to values
from the command line when executing a bakery build command. For more information on what options to use run help
for the bakery build command.