#!/bin/bash

set -e

RUN='cargo run --release --'

ALGS=(aes128-gcm chacha20-poly1305 none)
BACKUP_PARAMS=("--pass Pass --auth Auth --auth-every 1" "--pass Pass --auth Auth --auth-every 1" "")
CHECK_PARAMS=("--pass Pass" "--pass Pass" "")
RESTORE_PARAMS=("--pass Pass" "--pass Pass" "")

for i in ${!ALGS[@]}; do
    OUT_DIR=$(mktemp -d /tmp/bigarc.test.XXXXXX)
    dd if=/dev/urandom of=$OUT_DIR/src bs=4096 count=1024

    cat $OUT_DIR/src | \
        $RUN backup --out-template $OUT_DIR/%%% --alg ${ALGS[$i]} ${BACKUP_PARAMS[$i]} \
        --buf-size 16 --split-size 1 --compress-level 6 --no-check

    $RUN check   --config $OUT_DIR/000.cfg --buf-size 200 ${CHECK_PARAMS[$i]} 

    $RUN restore --config $OUT_DIR/000.cfg --buf-size 200 ${RESTORE_PARAMS[$i]} > $OUT_DIR/dst

    H1=$(sha1sum $OUT_DIR/src | cut -f1 -d' ')
    H2=$(sha1sum $OUT_DIR/dst | cut -f1 -d' ')
    #rm -rf $OUT_DIR
    [[ $H1 != $H2 ]] && ( echo Differs! ; exit 1 )
done
