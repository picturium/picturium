#!/bin/bash

docker volume create picturium-cargo-cache
docker run --rm -v "$(pwd):/root/picturium" -v picturium-cargo-cache:/root/.cargo/registry/cache -it --init lamka02sk/picturium-dev:8.15.1
