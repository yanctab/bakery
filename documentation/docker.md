# Introduction

By default, Bakery utilizes Docker and relies on the bakery-workspace image. Presently, Bakery has minimal requirements, only necessitating the availability of Docker on the system and that the user invoking Bakery is a member of the docker group. It's important to note that Bakery should never run as root. If you encounter permission issues when running Bakery with Docker, it's likely that the user is not part of the docker group.

# Setup Docker

There are multiple ways to install docker on your system and bakery has currently no preference. For reference on how to setup docker please see [setup-docker.sh](https://github.com/yanctab/bakery/blob/main/scripts/setup-docker.sh). This script currently supports setting up Docker on ubuntu and debian it is based on [Install Docker](https://docs.docker.com/engine/install/).

## Docker Group

Installing and correctly configuring Docker is a prerequisite for Bakery. It's essential that each user is a member of the docker group to avoid running Docker as root. For detailed instructions on [post-installation](https://docs.docker.com/engine/install/linux-postinstall/) steps on Linux, please refer to Post Install on Linux. Once Docker is set up, ensure that the user belongs to the docker group by running:


```bash
user@node:/dir$ groups
user adm cdrom sudo dip plugdev docker
```

# Bakery Workspace Image

The bakery workspace image can be pulled from Github Container Registry by running

```bash
user@node:/dir$ BAKERY_VERSION=$(bakery --version)
user@node:/dir$ BAKERY_VERSION=${BAKERY_VERSION##* }
user@node:/dir$ docker pull ghcr.io/yanctab/bakery/bakery-workspace:${BAKERY_VERSION}
```

Opening a shell to the bakery-workspace can be done by running

```bash
user@node:/dir$ docker run -it ghcr.io/yanctab/bakery/bakery-workspace:${BAKERY_VERSION} /bin/bash
```

## Custom Worksapce Image

If the default bakery workspace image is not enough a custome image can easily be setup. To get bakery to use the custom image change the values in the workspace config file

```json
        "docker": {
                "registry": "registry.io",
                "image": "custom-workspace",
                "tag": "x.y.z",
                "args": [
                ]
        }
```

When creating the custom bakery workspace make sure to include the following

```bash
# Setting up a bakery specific bash env pulled in by /etc/bash.bashrc
# if using bakery the /etc/bakery/bakery.bashrc will be mounted into
# docker so the file will exists, if not the sourcing will be ignored
RUN echo '[ -f /etc/bakery/bakery.bashrc ] && source /etc/bakery/bakery.bashrc' >> /etc/bash.bashrc
```

The bakery.bashrc will enable usage of the bakery aliases in the [bakery shell](sub-commands.md).

## Bootstrap Bakery

When starting bakery the first step is that bakery will bootstrap its self into the bakery-workspace image. The way this works is that
/usr/bin/bakery is mount binded inside docker using a docker --volume so the same bakery version is used inside docker as outside.
Bakery will also require to have access to the /etc/bakery/bakery.bashrc which is also mount binded to inside docker using the
--volume flag.



# Crops

The Yocto Project has established a Docker project named Crops. Although Bakery does not currently utilize it, integrating Crops containers is a future objective. These images are structured differently, and while there hasn't been an opportunity to explore their integration with Bakery yet, it is a desirable goal. Utilizing Crops containers could enhance Bakery's functionality and efficiency.

* https://hub.docker.com/r/crops/poky
* https://github.com/crops/yocto-dockerfiles
* https://github.com/crops/poky-container

