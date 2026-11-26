#!/bin/sh
# Re-check that this publish is still the tip of dev. The publish job runs this
# as its first step, after the job has entered its queue and before any write.
# A rerun of an older job, or an inverted pair of builds, must stop here.
set -eu
if [ "${CHANNEL:-}" != "dev" ]; then
  exit 0
fi
sha="${SHA:?SHA is required}"
tip="$(git fetch --no-tags origin dev && git rev-parse origin/dev)"
if [ "$sha" != "$tip" ]; then
  echo "dev publish is no longer the tip of dev (checkout $sha, tip $tip)" >&2
  exit 1
fi
echo "dev tip still $sha"
