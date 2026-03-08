#!/usr/bin/env bash
set -xeo pipefail

# ugly hack for cross to work on nixos
if [ -e /nix/store ]; then
    export NIX_STORE=/nix/store
fi

target_dir=$(cargo metadata --format-version=1 | jq -r .target_directory)
root=$(cargo metadata --format-version=1 | jq -r .resolve.root)
project_name=$(cargo metadata --format-version=1 \
               | jq -r ".packages[] | select(.id==\"${root}\") | .name")

# Build Linux (Cross-compiled on CentOS for max compatibility)
cross build --target=x86_64-unknown-linux-gnu --release
rm -rf dist_linux
mkdir -p dist_linux
cp "${target_dir}/x86_64-unknown-linux-gnu/release/${project_name}" dist_linux/

# Build Windows
nix build .#windows
rm -rf dist_windows
mkdir -p dist_windows
cp result/bin/${project_name}.exe dist_windows/
touch dist_windows/*

# Build WASM
env RELEASE=1 nix develop --command bash -c 'PATH="$HOME/.cargo/bin:$PATH" ./build_wasm.sh'

# Clean up symlink
rm -f result
