#!/bin/sh
# Write the Tauri config overlay for the compile-time release channel.
# The committed tauri.conf.json stays on the stable name and identifier.
set -eu
channel="${ANYBOT_RELEASE_CHANNEL:-stable}"
out="${1:-target/tauri-channel.json}"
mkdir -p "$(dirname "$out")"
case "$channel" in
  dev)
    product="AnyBot Dev"
    identifier="app.anybot.desktop.dev"
    ;;
  stable)
    product="AnyBot"
    identifier="app.anybot.desktop"
    ;;
  *)
    echo "ANYBOT_RELEASE_CHANNEL must be stable or dev" >&2
    exit 1
    ;;
esac
cat > "$out" <<EOF
{
  "productName": "$product",
  "identifier": "$identifier",
  "bundle": { "active": true }
}
EOF
echo "$out"
