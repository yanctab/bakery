FROM ubuntu:22.04

LABEL org.opencontainers.image.source=https://github.com/yanctab/bakery
LABEL org.opencontainers.image.description="Bakery Workspace"
LABEL org.opencontainers.image.licenses=Apache-2.0

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y \
        gawk \
        wget \
        git-core \
        subversion \
        diffstat \
        unzip \
        sysstat \
        texinfo \
        gcc-multilib \
        build-essential \
        gcc-11 \
	gcc-11-base \
	chrpath \
        socat \
        file \
	python-is-python3 \
        python3-pip \
        xz-utils  \
        locales \
        cpio \
        screen \
        tmux \
        sudo \
        iputils-ping \
        iproute2 \
        fluxbox \
        tig \
        vim \
        jq \
        apt-transport-https \
        ca-certificates \
        curl \
        gnupg \
        lsb-release \
        chrpath \
        diffstat \
        g++ \
        make \
        lz4 \
        zstd \
        python3-setuptools

RUN curl -fsSL https://download.docker.com/linux/ubuntu/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg && \
    echo "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu \
    $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null

RUN apt-get update && \
    apt-get install -y \
        docker-ce \
        docker-ce-cli \
        containerd.io

RUN dpkg-reconfigure locales && \
    locale-gen en_US.UTF-8 && \
    update-locale LC_ALL=en_US.UTF-8 LANG=en_US.UTF-8

ENV LC_ALL en_US.UTF-8
ENV LANG en_US.UTF-8
ENV LANGUAGE en_US.UTF-8

RUN curl -sS https://starship.rs/install.sh > starship-install.sh && \
    chmod 775 starship-install.sh && \
    ./starship-install.sh -y 

# Setting up the bakery bash env pulled in by /etc/bash.bashrc
# if using bakery the /etc/bakery/bkry.bashrc will be mounted into
# docker so the file will exists, if not the sourcing will be ignored
RUN echo '[ -f /etc/bakery/bkry.bashrc ] && source /etc/bakery/bkry.bashrc' >> /etc/bash.bashrc

RUN rm /bin/sh && \
    ln -s /bin/bash /bin/sh
