#!/bin/sh
# Push each native image by digest, then build the multi-arch tag from those digests.
# Dev and stable never share a tag: dev names are dev- and dev-<commit>.
# This script does not move :dev. The caller moves it only after the page swap.
# Shared arch-* and :verify tags are not used: two publishes can overwrite them.
# ANYBOT_DOCKER replaces docker in tests. Digests may be injected the same way.
# Annotations require an OCI index. Docker manifest lists reject them.
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
dir="${IMAGE_DIR:-target/image-parts}"
amd64="$(find "$dir" -name 'amd64.tar' -type f | head -n 1)"
arm64="$(find "$dir" -name 'arm64.tar' -type f | head -n 1)"
if [ -z "$amd64" ] || [ -z "$arm64" ]; then
  echo "missing a native image tar (amd64=$amd64 arm64=$arm64)" >&2
  exit 1
fi

push_one() {
  tar="$1"
  injected="$2"
  name="$3"
  if [ -n "$injected" ]; then
    printf '%s\n' "$injected"
    return
  fi
  # The tar keeps the runner-local tag from the build job. Retag to a name that
  # includes this commit, push that, and keep only the digest the registry returns.
  loaded="$("$docker" load -i "$tar")"
  ref="$(printf '%s\n' "$loaded" | sed -n 's/^Loaded image: //p' | head -n 1)"
  if [ -z "$ref" ]; then
    ref="$(printf '%s\n' "$loaded" | sed -n 's/^Loaded image ID: //p' | head -n 1)"
  fi
  if [ -z "$ref" ]; then
    echo "docker load did not name an image" >&2
    exit 1
  fi
  "$docker" tag "$ref" "$name"
  # Quiet mode is not documented to print the digest. The normal line is
  # "<tag>: digest: sha256:..." (docs.docker.com/reference/cli/docker/image/push).
  pushed="$("$docker" push "$name")"
  printf '%s\n' "$pushed" | sed -n 's/.*digest: //p' | awk '{print $1; exit}'
}

amd64_digest="$(push_one "$amd64" "${AMD64_DIGEST:-}" "${image}:${prefix}amd64-${short}")"
arm64_digest="$(push_one "$arm64" "${ARM64_DIGEST:-}" "${image}:${prefix}arm64-${short}")"
case "$amd64_digest" in
  sha256:*) ;;
  *)
    echo "amd64 push did not return a digest ($amd64_digest)" >&2
    exit 1
    ;;
esac
case "$arm64_digest" in
  sha256:*) ;;
  *)
    echo "arm64 push did not return a digest ($arm64_digest)" >&2
    exit 1
    ;;
esac

meta="${IMAGE_META:-target/image-index.json}"
mkdir -p "$(dirname "$meta")"
index_tag="${image}:${prefix}${short}"
# Dev does not apply :dev here. The caller moves it only after the page swap.
# Stable applies the version tag in this same create, so the draft's image
# exists before anyone publishes the Release. :latest stays with the caller.
# Sources must already be OCI. A Docker manifest list rejects index annotations
# (docs.docker.com/reference/cli/docker/buildx/imagetools/create/). The image
# job sets oci-mediatypes=true on the docker exporter. imagetools create has
# no format flag. Repeating --tag attaches every name to one index.
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
echo "pushed ${index_tag} ${digest}"
