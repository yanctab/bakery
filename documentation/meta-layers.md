# Introduction

One of the goals with Bakery is to simplify it for developers that have no or very little experience with Yocto/OE projects to get up and running but to setup a Bakery workspace some initial knowledge is required. When building a Yocto/OE project the meta layers are required. The meta layers contain meta data describing how to build images and all the packages that an image depends on. The meta layers are not something Bakery is handling so currently this is still a manual process where a developer
needs to decide what layers the build requires to build the desired image. Each layer could be its own git repo so some kind of solution is required to handle multiple repos. All the layers should be placed inside the layersdir in the Bakery workspace.
The layersdir can be defined in the [Workspace Config](workspace-config.md). Currently there are mainly two tools used to manage multiple repos

* git submodules
* repo android tool.

Most HW suppliers that supports Yocto/OE is using the repo android tool but in my opinion is git submodules more transparent and easier to follow. What tool to use is up to the project and Bakery has no requirements on the tool simply make sure that all meta layers are placed in the layersdir and that the workspace config is updated if the location of the layersdir is is changed. This document will not cover how to work and use these tools instead it will outline setup the workspace structure depending on the tool of preference.

# Git Submodules

For more information on git submodules see [Atlassian Git Submodules](https://www.atlassian.com/git/tutorials/git-submodule). In the Bakery workspace if using git submodules the entire workspace will be the main git repo and then additional git submodules will be added to the main git repo. So in the workspace example below we could have something like

```bash
    ├── .git
    ├── .gitmodules
    ├── .gitignore
    ├── artifacts
    ├── buildconfig.json
    ├── builds
    ├── .cache
    ├── docker
    ├── layers
    │   ├── meta-extra
    │   │   ├── .git
    │   │   ├── conf
    │   │   └── recipes-support
    │   └── poky
    │       ├── .git
    │       ├── bitbake
    │       ├── contrib
    │       ├── documentation
    │       ├── meta
    │       ├── meta-poky
    │       ├── meta-selftest
    │       ├── meta-skeleton
    │       ├── meta-yocto-bsp
    │       └── scripts
    ├── scripts
    └── workspace.json
```
where the poky and meta-extra directory is a git submodule. Make sure to add artifacts, builds, .cache to the gitignore file before starting a build.

# Android repo Tool

The Android repo tool is a python script developed by Google for the purpose of handling all the git repo required to build Android this was before git had support for git submodules so there was no other alternative. The concept is the same but instead of having a main repo containing the .gitmodules file and .git directory which togather contains all the information for managing the git submodule a seperate git repo is used. This git repo contains manifest files, each manifest files is a xml file containing a list of what additional git repos that the project requires. The workflow for using the repo tool or git submodules is similare except that there is no main repo. In my opinion it is mater of taste what you prefere and there are some pros and cons for both. In the workspace example bellow we could have something like

```bash
    ├── .repo
    ├── artifacts
    ├── buildconfig.json -> configs/buildconfig.json
    ├── builds
    ├── configs
    │   ├── .git
    │   ├── workspace.json
    │   └── buildconfig.json
    ├── .cache
    ├── docker
    │   ├── .git
    │   └── Dockerfile
    ├── layers
    │   ├── meta-extra
    │   │   ├── .git
    │   │   ├── conf
    │   │   └── recipes-support
    │   └── poky
    │       ├── .git
    │       ├── bitbake
    │       ├── contrib
    │       ├── documentation
    │       ├── meta
    │       ├── meta-poky
    │       ├── meta-selftest
    │       ├── meta-skeleton
    │       ├── meta-yocto-bsp
    │       └── scripts
    ├── scripts
    │   ├── .git
    │   └── test.sh
    └── workspace.json -> configs/workspace.json
```

Some things to note is that instead of a .git and .gitmodules we have .repo and all files directly placed in the workspace is links to some other directory which is a git repo. How this is setup will be define in the repo manifest.xml file.

# Migrate

To migrate between the two tools is not that hard all the information for what repos and what sha each repo should stand on for a project is available inside both tools it is then just a mater of a manual process or setting up a script to handle the migration. The details of the migration is outside of the scope for this documentation.
