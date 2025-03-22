#!/bin/bash

docker buildx build -t lamka02sk/picturium-dev:8.16.1 -f dev.yml --push --platform=linux/amd64 --progress=plain .
#docker buildx build -t lamka02sk/picturium-dev:8.16.1 -f dev.yml --load --platform=linux/amd64 --progress=plain .
