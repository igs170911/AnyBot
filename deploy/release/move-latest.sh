#!/bin/sh
# Point IMAGE:latest at the published non-rc version. Does not move a git tag.
set -eu

version="${TAG_NAME#v}"
case "$version" in
  *-*)
    echo "pre-release $TAG_NAME does not move latest" >&2
    exit 1
    ;;
esac
image="ghcr.io/$(printf '%s' "$REPOSITORY" | tr '[:upper:]' '[:lower:]')"
src="${image}:${version}"
docker buildx imagetools create --tag "${image}:latest" "$src"
