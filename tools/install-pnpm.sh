#!/bin/sh
# Install the pnpm version named in package.json, and check the tarball hash.
# The hash is the npm registry dist.integrity for pnpm@12.23.0, written the way
# Corepack writes packageManager (sha512.<hex>).
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$root"
want="$(python3 -c 'import json; print(json.load(open("package.json"))["packageManager"])')"
case "$want" in
  pnpm@12.23.0+sha512.*) ;;
  *)
    echo "packageManager is not the pinned pnpm@12.23.0 hash ($want)" >&2
    exit 1
    ;;
esac
hex="${want#pnpm@12.23.0+sha512.}"
url="https://registry.npmjs.org/pnpm/-/pnpm-12.23.0.tgz"
tmp="${RUNNER_TEMP:-$root/target}/pnpm-12.23.0.tgz"
mkdir -p "$(dirname "$tmp")"
curl -fsSL "$url" -o "$tmp"
got="$(python3 -c 'import hashlib,sys; print(hashlib.sha512(open(sys.argv[1],"rb").read()).hexdigest())' "$tmp")"
if [ "$got" != "$hex" ]; then
  echo "pnpm tarball sha512 mismatch" >&2
  exit 1
fi
npm install -g "$tmp"
pnpm --version
rm -f "$tmp"
