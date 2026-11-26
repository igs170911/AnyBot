#!/bin/sh
# Decide rehearsal vs publish, and refuse a tag that is not ready to publish.
# Writes GitHub Actions step outputs. Does not create directories.
# Tests put stub git and gh on PATH. BUILT_AT and GIT_DESCRIBE stay injectable
# because they are build inputs, not the publish decision.
set -eu

on_main() {
  sha="$1"
  git fetch --no-tags origin main
  git merge-base --is-ancestor "$sha" origin/main
}

dev_tip() {
  if [ -n "${DEV_HEAD_FILE:-}" ] && [ -f "$DEV_HEAD_FILE" ]; then
    tr -d '[:space:]' < "$DEV_HEAD_FILE"
    return
  fi
  git fetch --no-tags origin dev
  git rev-parse origin/dev
}

# CI workflow on this commit, head branch main. Linux, Windows, and macOS must
# all be success. A check suite has no workflow id, so the workflow run is the
# source (docs.github.com/en/rest/actions/workflow-runs, path and head_sha).
# Jobs: docs.github.com/en/rest/actions/workflow-jobs.
main_ci() {
  sha="$1"
  run_id="$(gh api --paginate \
    --method GET \
    -H "Accept: application/vnd.github+json" \
    "repos/${GITHUB_REPOSITORY}/actions/workflows/ci.yml/runs" \
    -f "head_sha=${sha}" \
    -F "exclude_pull_requests=true" \
    --jq '[.workflow_runs[] | select(.head_branch=="main" and .head_sha=="'"$sha"'")] | first | .id // ""')"
  case "$run_id" in
    ''|*[!0-9]*)
      printf '%s\n' "missing"
      return
      ;;
  esac
  names="$(gh api --paginate "repos/${GITHUB_REPOSITORY}/actions/runs/${run_id}/jobs" \
    --jq '[.jobs[] | select(.name=="check" or .name=="build-windows" or .name=="build-macos")] | map(select(.conclusion=="success")) | length')"
  if [ "$names" = "3" ]; then
    printf '%s\n' "success"
    return
  fi
  pending="$(gh api --paginate "repos/${GITHUB_REPOSITORY}/actions/runs/${run_id}/jobs" \
    --jq '[.jobs[] | select(.name=="check" or .name=="build-windows" or .name=="build-macos") | select(.status!="completed")] | length')"
  if [ "$pending" != "0" ]; then
    printf '%s\n' "running"
    return
  fi
  printf '%s\n' "failure"
}

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
image_tag="rehearsal"
# A reusable call keeps the caller's event name (push), so mode comes from the input.
# mode=dev publishes only when this call is the current tip of the dev branch.
if [ "$CALL_MODE" = "dev" ]; then
  mode="publish"
  channel="dev"
  tag="dev-build"
  image_tag="dev"
elif [ "$EVENT_NAME" = "workflow_dispatch" ]; then
  mode="rehearsal"
elif [ "$REF_TYPE" = "tag" ]; then
  mode="publish"
  channel="stable"
  tag="$REF_NAME"
  image_tag="${REF_NAME#v}"
fi
if [ "$mode" != "rehearsal" ] && [ "$mode" != "publish" ]; then
  echo "unknown mode $mode" >&2
  exit 1
fi

if [ "$CALL_MODE" = "dev" ]; then
  if [ "$REF_NAME" != "dev" ] || [ "$REF_TYPE" != "branch" ]; then
    echo "dev publish requires the caller to be the dev branch" >&2
    exit 1
  fi
  if [ "${CALL_COMMIT:-}" != "$SHA" ]; then
    echo "caller commit ${CALL_COMMIT:-missing} is not checkout $SHA" >&2
    exit 1
  fi
  dev_head="$(dev_tip)"
  if [ "$SHA" != "$dev_head" ]; then
    echo "dev publish requires the current tip of dev (checkout $SHA, tip $dev_head)" >&2
    exit 1
  fi
fi

if [ "$channel" = "stable" ]; then
  if [ "$tag" != "v$version" ] && ! printf '%s' "$tag" | grep -Eq "^v${version}-rc[.][0-9]+$"; then
    echo "tag $tag does not match workspace version $version" >&2
    exit 1
  fi
  if ! on_main "$SHA"; then
    echo "tag $tag is not on main" >&2
    exit 1
  fi
  # Only CI's own run on main counts. A wiki suite, or a green dev run of the
  # same commit, does not. Linux, Windows, and macOS must all be success.
  check="$(main_ci "$SHA")"
  case "$check" in
    success) ;;
    running)
      echo "main check for $SHA is not finished" >&2
      exit 1
      ;;
    *)
      echo "main check for $SHA is not success (got $check)" >&2
      exit 1
      ;;
  esac
fi

if [ -n "${BUILT_AT:-}" ]; then
  epoch="$BUILT_AT"
else
  epoch="$(git log -1 --format=%ct "$SHA")"
fi
if [ -n "${GIT_DESCRIBE:-}" ]; then
  describe="$GIT_DESCRIBE"
else
  describe="$(git describe --tags --always --abbrev=12 "$SHA")"
fi
# A tag-only describe repeats the tag and drops the commit (install-n10).
case "$describe" in
  *-g*) ;;
  *)
    short="$(printf '%s' "$SHA" | cut -c1-12)"
    describe="${describe}-g${short}"
    ;;
esac
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
  echo "image_tag=$image_tag"
} >> "${GITHUB_OUTPUT:?GITHUB_OUTPUT is required}"
