#!/bin/bash

set -e

function usage {
    printf "to prepare docker build image:\n\t$0 --image [os_ident]\n"
    printf "to build the application based on prviousely built image:\n\t$0 --app <branch> [os_ident]\n"
    exit 1
}

case $1 in 
    --image)
        OS=$2
        ARGS_MAX=2
        ;;
    --app)
        BRANCH=$2
        OS=$3
        ARGS_MAX=3
        if [ -z $BRANCH ]; then
            usage
        fi
        ;;
    *)
        usage
        exit 3
esac

if [ $# -gt $ARGS_MAX ]; then
    usage
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

    if [ -n "$OS" ]; then
        if [ "x$OS" != "x$IDENT" ]; then
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
            docker run -v=.:/src bigarchiver-$IDENT /bin/bash -l -c "/build-internal.sh $IDENT $BRANCH"
            ;;
    esac
    cd scripts
done

echo all done
