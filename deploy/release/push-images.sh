#!/bin/sh
# Build the multi-arch index from digests the image jobs already pushed.
# Those jobs use the registry exporter with oci-mediatypes=true. This script
# does not docker load or docker push: save/load rewrites OCI to a Docker
# manifest list, and index annotations are then rejected.
# https://docs.docker.com/build/metadata/annotations/
# https://docs.docker.com/build/exporters/image-registry/
# Dev and stable never share a tag: dev names are dev- and dev-<commit>.
# This script does not move :dev. The caller moves it only after the page swap.
# Shared arch-* and :verify tags are not used: two publishes can overwrite them.
# ANYBOT_DOCKER replaces docker in tests.
# https://docs.docker.com/reference/cli/docker/buildx/imagetools/create/
set -eu
docker="${ANYBOT_DOCKER:-docker}"
short="$(printf '%s' "$COMMIT" | cut -c1-12)"
moving="${IMAGE_TAG:?IMAGE_TAG is required}"
image="${IMAGE:?IMAGE is required}"
prefix=""
if [ "${CHANNEL:-}" = "dev" ]; then
  prefix="dev-"
  case "$moving" in
    dev) ;;
    *)
      echo "dev channel pushes only the dev tag name, not $moving" >&2
      exit 1
      ;;
  esac
fi
dir="${IMAGE_DIR:-target/image-digests}"

read_digest() {
  arch="$1"
  injected="$2"
  if [ -n "$injected" ]; then
    printf '%s\n' "$injected"
    return
  fi
  file="$(find "$dir" -name "${arch}.digest" -type f | head -n 1)"
  if [ -z "$file" ]; then
    echo "missing ${arch} digest file" >&2
    exit 1
  fi
  tr -d '[:space:]' < "$file"
}

amd64_digest="$(read_digest amd64 "${AMD64_DIGEST:-}")"
arm64_digest="$(read_digest arm64 "${ARM64_DIGEST:-}")"
case "$amd64_digest" in
  sha256:*) ;;
  *)
    echo "amd64 digest is not sha256 ($amd64_digest)" >&2
    exit 1
    ;;
esac
case "$arm64_digest" in
  sha256:*) ;;
  *)
    echo "arm64 digest is not sha256 ($arm64_digest)" >&2
    exit 1
    ;;
esac

meta="${IMAGE_META:-target/image-index.json}"
mkdir -p "$(dirname "$meta")"
index_tag="${image}:${prefix}${short}"
# Dev does not apply :dev here. The caller moves it only after the page swap.
# Stable applies the version tag in this same create, so the draft's image
# exists before anyone publishes the Release. :latest stays with the caller.
# Sources must already be OCI. A Docker manifest list rejects index annotations.
# imagetools create has no format flag. Repeating --tag attaches every name
# to one index.
version_tag=""
if [ -z "$prefix" ]; then
  case "$moving" in
    [0-9]*.[0-9]*.[0-9]*) version_tag="${image}:${moving}" ;;
    *)
      echo "stable channel requires a version tag, not $moving" >&2
      exit 1
      ;;
  esac
fi
"$docker" buildx imagetools create \
  --tag "$index_tag" \
  ${version_tag:+--tag "$version_tag"} \
  --annotation "index:org.opencontainers.image.revision=${COMMIT}" \
  --metadata-file "$meta" \
  "${image}@${amd64_digest}" \
  "${image}@${arm64_digest}"
if [ ! -s "$meta" ] && [ -n "${INDEX_DIGEST:-}" ]; then
  printf '%s\n' "{\"containerimage.descriptor\":{\"digest\":\"${INDEX_DIGEST}\"}}" > "$meta"
fi
digest="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["containerimage.descriptor"]["digest"])' "$meta")"
case "$digest" in
  sha256:*) ;;
  *)
    echo "imagetools metadata has no digest ($digest)" >&2
    exit 1
    ;;
esac
if [ -n "${GITHUB_OUTPUT:-}" ]; then
  printf 'digest=%s\n' "$digest" >> "$GITHUB_OUTPUT"
fi
echo "indexed ${index_tag} ${digest}"
