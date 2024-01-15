#!/bin/sh

set -e

if [ $# -ne 1 ]; then
    echo usage $0 \<src_prefix\>
    exit 1
fi

pwd
cd $1
pwd
tar xf sources.tar
cargo build --release && cargo test --release
