#!/bin/sh
# Point IMAGE:latest at a final vX.Y.Z that is not older than the current latest.
# The current version is the org.opencontainers.image.version annotation on :latest.
# Does not move a git tag. ANYBOT_DOCKER replaces docker in tests.
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
old="$version"
while :; do
  case "$old" in
    *.*) old="${old#*.}" ;;
    *) break ;;
  esac
done
case "$old" in
  ''|*[!0-9]*)
    echo "tag $tag_name is not v<number>.<number>.<number>" >&2
    exit 1
    ;;
esac

current="${LATEST_VERSION:-}"
if [ -z "$current" ]; then
  current="$("$docker" buildx imagetools inspect --format \
    '{{index .Manifest.Annotations "org.opencontainers.image.version"}}' \
    "${image}:latest" || true)"
fi
if [ -n "$current" ] && [ "$current" != "<no value>" ]; then
  newer="$(printf '%s\n%s\n' "$current" "$version" | sort -t. -k1,1n -k2,2n -k3,3n | tail -n 1)"
  if [ "$newer" != "$version" ] || [ "$version" = "$current" ]; then
    echo "refusing to move latest from $current back to $version" >&2
    exit 1
  fi
fi

"$docker" buildx imagetools create \
  --tag "${image}:latest" \
  --annotation "index:org.opencontainers.image.version=${version}" \
  "${image}:${version}"
echo "moved ${image}:latest to ${version}"
