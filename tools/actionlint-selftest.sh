#!/bin/bash
# A caller job that sets timeout-minutes must be blocked. A legal caller must pass.
set -euo pipefail
bin="${1:?actionlint}"
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/.github/workflows"
cat > "$work/.github/workflows/caller-timeout.yml" <<'EOF'
name: Bad caller
on: push
permissions:
  contents: read
jobs:
  call:
    uses: ./.github/workflows/other.yml
    timeout-minutes: 30
EOF
if "$bin" -color "$work/.github/workflows/caller-timeout.yml"; then
  echo "actionlint-selftest: timeout-minutes on a caller job was accepted" >&2
  exit 1
fi
cat > "$work/.github/workflows/caller-ok.yml" <<'EOF'
name: Good caller
on: push
permissions:
  contents: read
jobs:
  call:
    uses: ./.github/workflows/other.yml
    permissions:
      contents: read
EOF
"$bin" -color -ignore 'SC2016:' "$work/.github/workflows/caller-ok.yml"
echo "actionlint-selftest OK"
# Keep the real workflows in the message so a missing checkout is obvious.
test -f "$root/.github/workflows/ci.yml"
