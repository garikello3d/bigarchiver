#!/bin/bash

set -e

RUN='cargo run --release --'
OUT_DIR=$(mktemp -d /tmp/bigarc.test.XXXXXX)

dd if=/dev/urandom of=$OUT_DIR/src bs=4096 count=1024

cat $OUT_DIR/src | \
    $RUN backup --out-template $OUT_DIR/%%% --pass Pass \
    --buf-size 16 --auth Auth --auth-every 1 --split-size 1 --compress-level 6 --no-check

$RUN check   --config $OUT_DIR/000.cfg --pass Pass --buf-size 200

$RUN restore --config $OUT_DIR/000.cfg --pass Pass --buf-size 200 > $OUT_DIR/dst

H1=$(sha1sum $OUT_DIR/src | cut -f1 -d' ')
H2=$(sha1sum $OUT_DIR/dst | cut -f1 -d' ')
rm -rf $OUT_DIR
[[ $H1 != $H2 ]] && ( echo Differs! ; exit 1 )
