#!/bin/bash

cargo_toml_path="../Cargo.toml"

if [ ! -f "$cargo_toml_path" ]; then
    echo "Error: Cargo.toml file not found."
    exit 1
fi

version=$(grep -oP 'version\s*=\s*"\K[^"]+' "$cargo_toml_path" | head -n 1)

if [ -z "$version" ]; then
    echo "Error: Failed to extract version from Cargo.toml."
    exit 1
fi

echo "Building picturium v$version"
docker buildx build -t "lamka02sk/picturium:$version" -t lamka02sk/picturium:latest -f build.yml --push --platform=linux/amd64,linux/arm64 --progress=plain --build-context root=./../ .
# docker buildx build -t "lamka02sk/picturium:$version" -t lamka02sk/picturium:latest -f build.yml --load --platform=linux/amd64 --progress=plain --build-context root=./../ .
