#!/bin/sh
# Decide rehearsal vs publish, and refuse a tag that is not ready to publish.
# Writes GitHub Actions step outputs. Does not create directories.
set -eu

version="$(awk 'BEGIN{p=0} /^\[workspace\.package\]/{p=1; next} /^\[/{p=0} p && /^version[[:space:]]*=/{gsub(/"/, "", $3); print $3; exit}' Cargo.toml)"
tauri="$(python3 -c 'import json; print(json.load(open("apps/desktop/src-tauri/tauri.conf.json"))["version"])')"
desktop="$(python3 -c 'import json; print(json.load(open("apps/desktop/package.json"))["version"])')"
root="$(python3 -c 'import json; print(json.load(open("package.json"))["version"])')"
if [ "$version" != "$tauri" ] || [ "$version" != "$desktop" ] || [ "$version" != "$root" ]; then
  echo "version mismatch cargo=$version tauri=$tauri desktop=$desktop package=$root" >&2
  exit 1
fi

mode="rehearsal"
channel="dev"
tag="rehearsal"
# A reusable call keeps the caller's event name (push), so mode comes from the input.
if [ "$CALL_MODE" = "dev" ]; then
  mode="publish"
  channel="dev"
  tag="dev-build"
elif [ "$EVENT_NAME" = "workflow_dispatch" ]; then
  mode="rehearsal"
elif [ "$REF_TYPE" = "tag" ]; then
  mode="publish"
  channel="stable"
  tag="$REF_NAME"
fi
if [ "$mode" != "rehearsal" ] && [ "$mode" != "publish" ]; then
  echo "unknown mode $mode" >&2
  exit 1
fi

if [ "$channel" = "stable" ]; then
  if [ "$tag" != "v$version" ] && ! printf '%s' "$tag" | grep -Eq "^v${version}-rc[.][0-9]+$"; then
    echo "tag $tag does not match workspace version $version" >&2
    exit 1
  fi
  git fetch --no-tags origin main
  if ! git merge-base --is-ancestor "$SHA" origin/main; then
    echo "tag $tag is not on main" >&2
    exit 1
  fi
  # Actions jobs publish check runs, not commit statuses. The job name is `check`.
  check="$(gh api --paginate "repos/${GITHUB_REPOSITORY}/commits/${SHA}/check-runs" --jq '[.check_runs[] | select(.name=="check")] | first | .conclusion // ""')"
  if [ "$check" != "success" ]; then
    echo "commit $SHA has no successful check run named check (got ${check:-none}); refusing to publish" >&2
    exit 1
  fi
fi

if [ "$CALL_MODE" = "dev" ] && [ "${CALL_COMMIT:-}" != "$SHA" ]; then
  echo "caller commit ${CALL_COMMIT:-missing} is not checkout $SHA" >&2
  exit 1
fi

epoch="$(git log -1 --format=%ct "$SHA")"
describe="$(git describe --tags --always --abbrev=12 "$SHA")"
case "$describe" in
  *-dirty*)
    echo "refusing a dirty describe ($describe)" >&2
    exit 1
    ;;
esac

image="ghcr.io/$(printf '%s' "$REPOSITORY" | tr '[:upper:]' '[:lower:]')"
case "$image" in
  *[!a-z0-9._:/-]*)
    echo "image name is not a registry ref ($image)" >&2
    exit 1
    ;;
esac

{
  echo "mode=$mode"
  echo "channel=$channel"
  echo "version=$version"
  echo "tag=$tag"
  echo "commit=$SHA"
  echo "built_at=$epoch"
  echo "describe=$describe"
  echo "image=$image"
} >> "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"
