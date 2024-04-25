#!/bin/bash

docker buildx build -t lamka02sk/picturium-base:8.15.1 -f base.yml --push --platform=linux/amd64,linux/arm64 --progress=plain .
# docker buildx build -t lamka02sk/picturium-base:8.15.1 -f base.yml --load --platform=linux/amd64 --progress=plain .
