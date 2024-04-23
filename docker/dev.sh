#!/bin/bash

docker buildx build -t lamka02sk/picturium-dev:8.15.1 -f dev.yml --push --platform=linux/amd64 --progress=plain .
