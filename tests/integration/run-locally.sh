#!/bin/bash

set -e

cd $(dirname $(readlink -f $0))

export WORKSPACE=$(readlink -f ../../..)/workspace

git clone https://github.com/sstephenson/bats/ || true
export PATH=$(pwd)/bats/bin:$(readlink -f ../../target/debug):$PATH

rm -rf $WORKSPACE
mkdir $WORKSPACE
cp -ar fixtures/* $WORKSPACE

bats integration.bats
