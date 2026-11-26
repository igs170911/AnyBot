#!/bin/sh
# Write the compile-time release values into .cargo/config.toml for this worktree.
# CI calls export-build-env.sh instead and passes the same three names as env.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
cd "$root"
# shellcheck disable=SC1091
eval "$("$root/deploy/release/build-env.sh" | sed 's/^/export /')"
mkdir -p "$root/target"
cat > "$root/target/release-cargo-config.toml" <<EOF
[env]
ANYBOT_RELEASE_CHANNEL = "${ANYBOT_RELEASE_CHANNEL}"
ANYBOT_GIT_DESCRIBE = "${ANYBOT_GIT_DESCRIBE}"
SOURCE_DATE_EPOCH = "${SOURCE_DATE_EPOCH}"
EOF
echo "stamped $root/target/release-cargo-config.toml ($ANYBOT_RELEASE_CHANNEL $ANYBOT_GIT_DESCRIBE)"
echo "pass it with: cargo build --config target/release-cargo-config.toml"
