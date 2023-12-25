#!/bin/bash

set -e

if [ $# -eq 0 ]; then
    echo usage $0 --image\|--app [ os_ident ]
    exit 1
fi

cat PLATFORMS | sed '/^#/d;/^[[:space:]]*$/d' | while read LINE; do
    WORDS=($LINE)
    if [ ${#WORDS[@]} -ne 3 ]; then
        echo Invalid PLATFORMS file
        exit 1
    fi

    IDENT=${WORDS[0]}
    IMAGE_FROM=${WORDS[1]}
    PAC_MGR=${WORDS[2]}

    if [ -n "$2" ]; then
        if [ x"$IDENT" != x"$2" ] ; then
            continue
        fi
    fi

    cd ..
    case $1 in 
        --image)
            echo preparing build image for $IDENT
            docker build -t bigarchiver-$IDENT -f scripts/Dockerfile.template --build-arg OS=$IMAGE_FROM --build-arg PAC=$PAC_MGR .
            ;;
        --app)
            echo building application for $IDENT
            docker run -v=.:/src bigarchiver-$IDENT /bin/bash -l -c "/build-internal.sh $IDENT"
            ;;
        *)
            echo invalid usage
            exit 3
    esac
    cd scripts
done

echo all done
