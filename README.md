[![Build & Test](https://github.com/yanctab/bakery/actions/workflows/buildntest.yml/badge.svg)](https://github.com/yanctab/bakery/actions/workflows/buildntest.yml) [![Release](https://github.com/yanctab/bakery/actions/workflows/release.yml/badge.svg)](https://github.com/yanctab/bakery/actions/workflows/release.yml)

# Bakery: Build Engine for the Yocto/OE Projects

## Introduction

Bakery is a versatile command-line tool designed to simplify and streamline the process of building Yocto Project or OpenEmbedded projects. It serves as a wrapper around the powerful bitbake tool, leveraging Docker to ensure consistent environments across local and CI (Continuous Integration) builds.

### Why Bakery?

Building complex Yocto Project or OpenEmbedded projects can often involve intricate scripts and directory structures, making maintenance challenging. Bakery addresses this challenge by introducing a standardized approach while preserving flexibility. It aims to enhance the development experience by consolidating essential project configuration into JSON files, promoting a cleaner and more maintainable workflow.

## Installation

### Debian Package

Download one of the deb-packages releases.

```bash
user@node:/dir$ BAKERY_VERSION=x.y.z
user@node:/dir$ wget https://github.com/yanctab/bakery/releases/download/v${BAKERY_VERSION}/bakery-x86_64-musl-v${BAKERY_VERSION}.deb
user@node:/dir$ sudo dpkg -i bakery-x86_64-musl-v${BAKERY_VERSION}.deb
```

Because bakery is written in Rust and libc musl bakery is a single binary. It will be installed under /usr/bin.

## Usage

```bash
Build engine for the Yocto/OE

Usage: bakery <COMMAND>

Commands:
  sync    Sync workspace e.g sync/update git submodules
  upload  Upload artifacts to artifactory server
  deploy  Deploy artifact to target
  list    List all builds or the tasks available for one build
  setup   Setup workspace e.g initializing git submodules
  clean   Clean one or all the tasks defined in a build config
  build   Execute a build either a full build or a task of one of the builds
  shell   Initiate a shell within Docker or execute any command within the BitBake environment
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

```

For detailed instructions on using each sub-command, please refer to the [sub-commands](documentation/sub-commands.md) documentation. Each sub-command is configured by defining a build configuration file that describes how to build a product. It is recommended to begin by starting a shell, as this not only sets up the environment for using Bakery but also enables the use of any tools available through OE/Yocto.


```bash
user@node:/dir$ bakery shell -c <config>
```

When starting a Bakery shell the config will be selected and used inside the shell. The terminal will present the following information

```bash
<user>@bakery-v<version>[<config>]:~/$
```

Each subcommand will be available as an alias with the build config predefined. Simply type the subcommand from any location to run it. To get a list of the current subcommands run


```bash
<user>@bakery-v<version>[<config>]:~/$ help
```

To start a build run

```bash
<user>@bakery-v<version>[<config>]:~/$ build
```

For more information on how to use the shell and any sub-command please refere to [shell](documentation/sub-commands.md#shell).

## Eyecandy

Powered by [starship](https://starship.rs/) requires that your GUI terminal has setup [nerdfonts](https://www.nerdfonts.com/). To enable run

```bash
bakery shell -c <build config> --eyecandy
```

## Build Source Code

Please see [build source code](documentation/build-bakery.md) for information on how to build bakery.

## Docker

By default, Bakery utilizes Docker. Refer to the [Docker setup guide](documentation/docker.md) for detailed instructions on setting up Docker on your host system. Additionally, if you wish to run Bakery without Docker, please consult the guide on [disabling Docker](documentation/workspace-config.md#disabled) for detailed instructions. To test out that docker works run


```bash
user@node:/dir$ BAKERY_VERSION=x.y.z
user@node:/dir$ docker run -it ghcr.io/yanctab/bakery/bakery-workspace:${BAKERY_VERSION} /bin/bash
```

## Template Project

To try bakery out the easiest way is to use the template workspace under tests/template-workspace. Run

```bash
user@node:/dir$ cd tests/template-workspace
user@node:/dir$ git submodule update
```

This will pull down poky and together with the workspace and the build config beaglebone black can be built using the bakery workspace shell.

```bash
user@bakery-vx.y.z[beaglebone]:~/$
```

## Setup A Project

To setup a project from scratch mainly four things are required

* bakery - the bakery tool. Please see [Installation](#Installation)
* workspace - the workspace config defining the workspace structure. Please see [Setup Workspace](documentation/workspace-config.md).
* build config - the build config defining how to build a product. Please see [Build Config](documentation/build-config.md).
* meta layers - the meta data used by bitbake when producing the artifacts needed by the product. Please see [Meta Layers](documentation/meta-layers.md).

## Key Features

- **Docker Integration:** Bakery seamlessly integrates with Docker to create reproducible build environments, ensuring consistency across different development setups.

- **JSON Configuration:** Project-specific configuration is defined in JSON files – the build config and the workspace config. The build config encapsulates all the necessary local.conf and bblayers.conf settings, simplifying the build process.

- **Task-Based Workflow:** Bakery organizes tasks within a build, allowing users to define various operations, from building image recipes to signing firmware or packaging images for redistribution. Tasks can be BitBake tasks or custom scripts, providing flexibility in project workflows.

- **Developer-Centric CI Alignment:** Bakery promotes a development environment where CI builds are defined using tasks in the build config. This ensures that CI processes are transparent and understandable, allowing developers to effortlessly reproduce the CI build locally.

    - **Consistency Across Environments:** With Bakery, the CI build process is aligned with local builds, promoting consistency. Developers can replicate CI builds effortlessly, reducing the chances of discrepancies between development and CI environments.

    - **Debugging Made Easy:** Developers can quickly identify and resolve issues by reproducing the exact CI build locally. This tight integration between CI and local development simplifies debugging, leading to faster issue resolution.

    - **Improved Developer Experience:** By aligning CI with local builds, Bakery enhances the overall developer experience. Developers gain confidence in the reliability of their builds and can iterate more efficiently on their projects.

### What Bakery Does Not Do

While Bakery simplifies many aspects of Yocto Project and OpenEmbedded development, it's essential to understand its limitations:

- **Meta Layer Setup:** Bakery does not handle the initial setup of meta layers. Developers are encouraged to use Git submodules or the Android Repo tool for managing meta layers independently.
- **Tool Replacement:** Bakery does not replace BitBake or any other tools available for Yocto Project or OpenEmbedded projects. It complements existing tools by wrapping some complexity to make it more straightforward for users with varying levels of knowledge.

Bakery strives to maintain the flexibility for developers who seek complete control of their projects, ensuring compatibility with all tools and workflows available in a Yocto Project and OpenEmbedded project.

