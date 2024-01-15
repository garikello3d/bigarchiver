set -e

if [ $# -ne 1 ]; then
    echo usage: $0 \<package install command\>
    exit 1
fi

$1 curl git gcc

if [[ $(cargo version) ]] ; then
    echo cargo installed, skipping
else
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > /tmp/rustup && sh /tmp/rustup -y
fi

cd dummy
echo "// dummy file" > src/lib.rs
cargo build --release
