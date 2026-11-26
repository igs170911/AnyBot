#!/bin/sh
# Rebuild the placeholder desktop icons from a generated 1024 PNG.
# Requires pnpm and the desktop workspace's @tauri-apps/cli. Does not copy icons.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/../.." && pwd)"
python3 "$root/deploy/release/gen-placeholder-icon.py"
src="$root/target/placeholder-icon.png"
test -s "$src"
cd "$root/apps/desktop"
pnpm exec tauri icon "$src"
echo "icons written under apps/desktop/src-tauri/icons"
