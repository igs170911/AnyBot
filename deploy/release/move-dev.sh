#!/bin/sh
# Point IMAGE:dev at the commit-scoped index this publish just pushed.
# Run only after the dev page swap succeeded. A page failure must not call this.
# Does not move a git tag. ANYBOT_DOCKER replaces docker in tests.
set -eu
docker="${ANYBOT_DOCKER:-docker}"
image="${IMAGE:?IMAGE is required}"
commit="${COMMIT:?COMMIT is required}"
short="$(printf '%s' "$commit" | cut -c1-12)"
"$docker" buildx imagetools create \
  --tag "${image}:dev" \
  "${image}:dev-${short}"
echo "moved ${image}:dev to dev-${short}"
