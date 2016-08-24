#!/bin/bash

set -e

if [[ `uname` == 'Darwin' ]]; then
  hash greadlink 2>/dev/null || \
  hash gcp 2>/dev/null || \
  { echo >&2 "you're running OS X:
  to make this script compatible coreutils  is required;
  install with 'brew install coreutils'; Aborting."; exit 1; }
  readlink() {
    $(which greadlink) "$@"
  }
  cp() {
    $(which gcp) "$@"
  }
fi

cd $(dirname $(readlink -f $0))

export WORKSPACE=$(readlink -f ../../..)/workspace

git clone https://github.com/sstephenson/bats/ || true
export PATH=$(pwd)/bats/bin:$(readlink -f ../../target/debug):$PATH

rm -rf $WORKSPACE
mkdir $WORKSPACE
cp -ar fixtures/* $WORKSPACE

bats integration.bats
