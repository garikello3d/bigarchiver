#!/bin/bash

set -e

if [ $# -ne 2 ]; then
    echo usage: $0 \<remote_addr\> \<package install command\> 
    exit 1
fi
HOST=$1
PAC=$2

scp scripts/install-internal.sh $HOST:
scp scripts/build-internal.sh $HOST:
ssh $HOST -- "mkdir -pv dummy/src"
scp Cargo.toml $HOST:dummy
ssh $HOST -- "bash -c './install-internal.sh \"$PAC\"'"
