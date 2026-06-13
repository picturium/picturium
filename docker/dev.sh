#!/bin/bash

# docker buildx build -t lamka02sk/picturium-dev:8.18.0 -f dev.yml --push --platform=linux/amd64 --progress=plain .
docker buildx build --builder=default -t lamka02sk/picturium-dev:8.18.2 -f dev.yml --load --progress=plain .
