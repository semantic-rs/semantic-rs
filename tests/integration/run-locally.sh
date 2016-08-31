#!/bin/bash

set -e

expandpath() {
  CDPATH="" cd -- "$1" && pwd -P
}

SCRIPTPATH=$(expandpath "$(dirname -- "$0")")
cd "$SCRIPTPATH"

export WORKSPACE
WORKSPACE="$(expandpath ../../..)/workspace"

git clone https://github.com/sstephenson/bats/ || true
export PATH
PATH=$(pwd)/bats/bin:$(expandpath ../../target/debug):$PATH

rm -rf "$WORKSPACE"
mkdir "$WORKSPACE"
cp -aR fixtures/* "$WORKSPACE"

bats integration.bats
