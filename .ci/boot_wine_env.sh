#!/usr/bin/env bash

# Install add-apt-repository
apt update
apt -y install software-properties-common dirmngr apt-transport-https lsb-release ca-certificates

# Install gcc
apt -y install build-essential

# Install other tools
apt -y install wget curl

# Add Wine PPA
dpkg --add-architecture i386
wget -qO - https://dl.winehq.org/wine-builds/winehq.key | apt-key add -
apt-add-repository -y 'deb https://dl.winehq.org/wine-builds/ubuntu/ bionic main'

# Install wine and cross-toolchain
apt-get install -y --install-recommends winehq-stable
apt-get install -y mingw-w64

wine --version

