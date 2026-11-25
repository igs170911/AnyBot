#!/bin/sh
# Push each native image, then point one tag at both architectures.
# The tars were built on ubuntu-24.04 and ubuntu-24.04-arm. This step does not rebuild.
set -eu
short="$(printf '%s' "$COMMIT" | cut -c1-12)"
if [ "$CHANNEL" = "dev" ]; then
  moving="dev"
else
  moving="$VERSION"
fi
dir="target/image-parts"
amd64="$(find "$dir" -name 'amd64.tar' -type f | head -n 1)"
arm64="$(find "$dir" -name 'arm64.tar' -type f | head -n 1)"
if [ -z "$amd64" ] || [ -z "$arm64" ]; then
  echo "missing a native image tar (amd64=$amd64 arm64=$arm64)" >&2
  exit 1
fi
docker load -i "$amd64"
docker tag "${IMAGE}:verify" "${IMAGE}:arch-amd64"
docker push "${IMAGE}:arch-amd64"
docker load -i "$arm64"
docker tag "${IMAGE}:verify" "${IMAGE}:arch-arm64"
docker push "${IMAGE}:arch-arm64"
for name in "$moving" "$short"; do
  docker buildx imagetools create \
    --tag "${IMAGE}:${name}" \
    "${IMAGE}:arch-amd64" \
    "${IMAGE}:arch-arm64"
done
echo "pushed ${IMAGE}:${moving} and ${IMAGE}:${short}"
