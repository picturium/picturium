#!/bin/bash

docker volume create picturium-cargo-cache
docker run --rm -v "$(pwd):/root/picturium" -v "$(pwd)/../picturium-libvips:/root/picturium-libvips" -v picturium-cargo-cache:/root/.cargo/registry/cache \
  -v /usr/share/fonts:/usr/share/fonts --init -p 20046:20046 --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
  lamka02sk/picturium-dev:8.18.2
