#!/bin/sh
# Point IMAGE:latest at a final vX.Y.Z that is not older than the current latest.
# The current version is the tag name of the image the :latest index lists.
# That tag is applied in the same imagetools create as the commit index, so it
# does not depend on an annotation surviving a Docker manifest list.
# Only an explicit missing-tag error from the registry counts as a first publish.
# Any other read error stops and does not move. Does not move a git tag.
# ANYBOT_DOCKER replaces docker in tests.
# https://docs.docker.com/reference/cli/docker/buildx/imagetools/inspect/
set -eu
docker="${ANYBOT_DOCKER:-docker}"
tag_name="${TAG_NAME:?TAG_NAME is required}"
repository="${REPOSITORY:?REPOSITORY is required}"
image="ghcr.io/$(printf '%s' "$repository" | tr '[:upper:]' '[:lower:]')"

case "$tag_name" in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *)
    echo "tag $tag_name is not v<number>.<number>.<number>; refusing to move latest" >&2
    exit 1
    ;;
esac
version="${tag_name#v}"
case "$version" in
  *[!0-9.]*)
    echo "tag $tag_name is not a final version; refusing to move latest" >&2
    exit 1
    ;;
esac

part=0
rest="$version"
while [ -n "$rest" ]; do
  case "$rest" in
    *.*) piece="${rest%%.*}"; rest="${rest#*.}" ;;
    *) piece="$rest"; rest="" ;;
  esac
  part=$((part + 1))
  case "$piece" in
    ''|*[!0-9]*|0[0-9]*)
      echo "tag $tag_name is not <digits>.<digits>.<digits> without leading zeros" >&2
      exit 1
      ;;
  esac
done
if [ "$part" -ne 3 ]; then
  echo "tag $tag_name is not exactly three numeric parts" >&2
  exit 1
fi

current="${LATEST_VERSION:-}"
if [ -z "${LATEST_VERSION+x}" ]; then
  err="${LATEST_ERR:-target/latest-inspect.err}"
  mkdir -p "$(dirname "$err")"
  # imagetools inspect prints one platform per line. The version is the
  # tag on that platform's source, not an annotation on :latest.
  if "$docker" buildx imagetools inspect "${image}:latest" >"$err.out" 2>>"$err"; then
    current="$(sed -n 's/.*tag: //p' "$err.out" | head -n 1)"
  elif err_text="$(cat "$err")" && case "$err_text" in
    *'manifest unknown'*|*'not found'*|*'no such manifest'*|*NAME_UNKNOWN*|*MANIFEST_UNKNOWN*) true ;;
    *) false ;;
  esac; then
    current=""
  else
    echo "cannot read ${image}:latest; refusing to move" >&2
    exit 1
  fi
fi
if [ -n "$current" ] && [ "$current" != "<no value>" ]; then
  newer="$(printf '%s\n%s\n' "$current" "$version" | sort -t. -k1,1n -k2,2n -k3,3n | tail -n 1)"
  if [ "$newer" != "$version" ] || [ "$version" = "$current" ]; then
    echo "refusing to move latest from $current back to $version" >&2
    exit 1
  fi
fi

# No index annotation. :latest names the version tag, which already exists.
"$docker" buildx imagetools create \
  --tag "${image}:latest" \
  "${image}:${version}"
echo "moved ${image}:latest to ${version}"
