#!/bin/bash

docker volume create picturium-cargo-cache
docker run --rm -v "$(pwd):/root/picturium" -v picturium-cargo-cache:/root/.cargo/registry/cache -v /usr/share/fonts:/usr/share/fonts -it --init -p 20045:20045 lamka02sk/picturium-dev:8.16.1 cargo test $1
