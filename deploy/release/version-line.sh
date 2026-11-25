#!/bin/sh
# Print the install-n10 version line from the values export-build-env.sh wrote.
set -eu
version="$(awk 'BEGIN{p=0} /^\[workspace\.package\]/{p=1; next} /^\[/{p=0} p && /^version[[:space:]]*=/{gsub(/"/, "", $3); print $3; exit}' Cargo.toml)"
describe="${ANYBOT_GIT_DESCRIBE:?ANYBOT_GIT_DESCRIBE is required}"
epoch="${SOURCE_DATE_EPOCH:?SOURCE_DATE_EPOCH is required}"
commit="$describe"
dirty=""
case "$describe" in
  *-dirty)
    commit="${describe%-dirty}"
    dirty="-dirty"
    ;;
esac
case "$commit" in
  *-g*) short="${commit##*-g}" ;;
  *) short="$commit" ;;
esac
stamp="$(python3 -c 'import os, time; print(time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(int(os.environ["SOURCE_DATE_EPOCH"]))))')"
printf 'v%s-%s%s · %s\n' "$version" "$short" "$dirty" "$stamp"
