#!/usr/bin/env bash
# selftest of the api-compat gate (spec/09 K86): every scenario is a scratch git repo
# with fixture baselines; the tool must block the bad ones and pass the good ones.
set -uo pipefail

repo=${1:-.}
scratch="$repo/target/scratch/api-compat-selftest"
bin="$repo/target/debug/api-compat"

failures=0

say() {
	echo "api-compat-selftest: $*"
}

expect() {
	local name="$1" want="$2" needle="$3" out="$4" rc="$5"
	if [ "$want" = "fail" ] && [ "$rc" -eq 0 ]; then
		say "FAIL: $name passed but must block"
		failures=$((failures + 1))
		return
	fi
	if [ "$want" = "pass" ] && [ "$rc" -ne 0 ]; then
		say "FAIL: $name blocked but must pass (output: $out)"
		failures=$((failures + 1))
		return
	fi
	if [ -n "$needle" ] && ! printf '%s' "$out" | grep -Fq "$needle"; then
		say "FAIL: $name output lacks '$needle'"
		failures=$((failures + 1))
		return
	fi
	say "ok: $name"
}

run_tool() {
	local name="$1" want="$2" needle="$3" ref="$4"
	local out rc
	out=$("$bin" --repo "$scratch" --against "$ref" 2>&1)
	rc=$?
	expect "$name" "$want" "$needle" "$out" "$rc"
}

seed_baseline() {
	cat "$scratch/api/openapi.yaml" "$scratch/api/asyncapi.yaml" | shasum -a 256 | cut -d' ' -f1
}

commit_all() {
	git -C "$scratch" add -A
	git -C "$scratch" -c user.email=selftest@invalid -c user.name=selftest commit -m "$1" >/dev/null
}

rm -rf "$scratch"
mkdir -p "$scratch/api"
git -C "$scratch" init -b main >/dev/null
cp tests/api-compat-selftest/fixtures/openapi.yaml "$scratch/api/openapi.yaml"
cp tests/api-compat-selftest/fixtures/asyncapi-base.yaml "$scratch/api/asyncapi.yaml"
seed_sha=$(seed_baseline)
printf '%s | sha256=%s | fixtures seeded | the selftest starts from a committed baseline\n' "2026-01-01" "$seed_sha" > "$scratch/api/breaking-changes.log"
commit_all "seed"
cp tests/api-compat-selftest/fixtures/asyncapi-added.yaml "$scratch/api/asyncapi.yaml"
grown_sha=$(seed_baseline)
printf '%s | sha256=%s | a property was added | additive change for scenario four\n' "2026-01-02" "$grown_sha" >> "$scratch/api/breaking-changes.log"
commit_all "added"

cargo build --locked -p api-compat --quiet

cp tests/api-compat-selftest/fixtures/asyncapi-breaking.yaml "$scratch/api/asyncapi.yaml"
run_tool "breaking change blocks" fail "breaking protocol change" HEAD

git -C "$scratch" checkout HEAD~1 -- api
run_tool "unchanged tree passes" pass "" HEAD~1

cp tests/api-compat-selftest/fixtures/asyncapi-added.yaml "$scratch/api/asyncapi.yaml"
head -n 1 "$scratch/api/breaking-changes.log" > "$scratch/api/tmp.log" && mv "$scratch/api/tmp.log" "$scratch/api/breaking-changes.log"
run_tool "baseline change without a record blocks" fail "breaking-changes.log must grow" HEAD~1

cp tests/api-compat-selftest/fixtures/asyncapi-added.yaml "$scratch/api/asyncapi.yaml"
head -n 1 "$scratch/api/breaking-changes.log" > "$scratch/api/tmp.log" && mv "$scratch/api/tmp.log" "$scratch/api/breaking-changes.log"
printf '%s | sha256=%s | a property was added | additive change for scenario four\n' "2026-01-02" "$grown_sha" >> "$scratch/api/breaking-changes.log"
run_tool "recorded additive change passes" pass "" HEAD~1

printf '%s\n' "garbage line" > "$scratch/api/breaking-changes.log"
cp tests/api-compat-selftest/fixtures/asyncapi-base.yaml "$scratch/api/asyncapi.yaml"
run_tool "malformed log line blocks" fail "not 64 hex digits" HEAD~1

git -C "$scratch" checkout HEAD~1 -- api
cp tests/api-compat-selftest/fixtures/asyncapi-breaking.yaml "$scratch/api/asyncapi.yaml"
breaking_sha=$(seed_baseline)
printf '%s | sha256=%s | a payload property was removed | recorded removal still blocks\n' "2026-01-03" "$breaking_sha" >> "$scratch/api/breaking-changes.log"
run_tool "recorded breaking change still blocks" fail "breaking protocol change" HEAD~1

git -C "$scratch" checkout HEAD -- api
run_tool "committed state passes" pass "" HEAD

if [ "$failures" -ne 0 ]; then
	say "FAIL: $failures scenario(s) blocked incorrectly"
	exit 1
fi
say "api-compat-selftest OK"
