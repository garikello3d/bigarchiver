#!/bin/bash

if [ $# -ne 2 ]; then
    echo usage $0 \<os_ident\> \<branch\>
    exit 1
fi

cd /
git clone --branch $2 --single-branch /src/ bigarchiver && \
cd bigarchiver/ && \
cargo test --release && cargo build --release && \
mkdir -pv /src/scripts/build/$1/ && \
cp -v target/release/bigarchiver /src/scripts/build/$1/
