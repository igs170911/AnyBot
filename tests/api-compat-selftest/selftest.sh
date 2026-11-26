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
	if { [ "$want" = "fail" ] && [ "$rc" -eq 0 ]; } || { [ "$want" = "pass" ] && [ "$rc" -ne 0 ]; }; then
		if [ "$want" = "pass" ]; then
			say "FAIL: $name blocked but must pass (output: $out)"
		else
			say "FAIL: $name passed but must block"
		fi
		failures=$((failures + 1))
		return 0
	fi
	if [ -n "$needle" ]; then
		local found=0
		if printf '%s' "$out" | grep -Fq "$needle"; then
			found=1
		fi
		if [ "$found" -eq 0 ]; then
			say "FAIL: $name output lacks '$needle'"
			failures=$((failures + 1))
			return 0
		fi
	fi
	say "ok: $name"
	return 0
}

run_tool() {
	local name="$1" want="$2" needle="$3"
	local out rc
	out=$("$bin" --repo "$scratch" --against "$ref" 2>&1)
	rc=$?
	expect "$name" "$want" "$needle" "$out" "$rc"
}

commit_all() {
	git -C "$scratch" add -A
	git -C "$scratch" -c user.email=selftest@invalid -c user.name=selftest commit -m "$1" >/dev/null
	if [ "$?" -ne 0 ]; then
		say "FAIL: git commit $1 failed"
		exit 1
	fi
}

baseline_sha() {
	cat "$scratch/api/openapi.yaml" "$scratch/api/asyncapi.yaml" | shasum -a 256 | cut -d' ' -f1
}

fixture() {
	cp "tests/api-compat-selftest/fixtures/$1" "$scratch/api/$2"
}

start_scenario() {
	fixture openapi.yaml openapi.yaml
	fixture asyncapi-base.yaml asyncapi.yaml
	printf '%s | sha256=%s | fixtures seeded | the selftest starts from a committed baseline\n' "2026-01-01" "$(baseline_sha)" > "$scratch/api/breaking-changes.log"
	commit_all "scenario seed"
	ref=$(git -C "$scratch" rev-parse HEAD)
}

append_record() {
	printf '%s | sha256=%s | %s | %s\n' "$1" "$(baseline_sha)" "$2" "$3" >> "$scratch/api/breaking-changes.log"
}

rm -rf "$scratch"
mkdir -p "$scratch/api"
git -C "$scratch" init -b main >/dev/null
git -C "$scratch" -c user.email=selftest@invalid -c user.name=selftest commit --allow-empty -m "initial empty commit" >/dev/null
start_scenario
printf '%s\n' "# a trailing comment line: the files themselves do not change" >> "$scratch/api/breaking-changes.log"
commit_all "touch the changelog comment"
ref=$(git -C "$scratch" rev-parse HEAD)

cargo build --locked -p api-compat --quiet
build_rc=$?
if [ "$build_rc" -ne 0 ]; then
	say "FAIL: cargo build -p api-compat failed (rc=$build_rc); refusing to run an older binary"
	exit 1
fi

start_scenario
fixture asyncapi-breaking.yaml asyncapi.yaml
commit_all "remove a payload property"
run_tool "breaking change blocks" fail "breaking-changes.log must grow"

start_scenario
append_touch() {
	printf '%s\n' "# a trailing comment line: the files themselves do not change" >> "$scratch/api/breaking-changes.log"
}
append_touch
commit_all "touch the changelog comment"
run_tool "unchanged tree passes" pass ""

start_scenario
fixture asyncapi-added.yaml asyncapi.yaml
commit_all "add a property without a record"
run_tool "baseline change without a record blocks" fail "breaking-changes.log must grow"

start_scenario
fixture asyncapi-added.yaml asyncapi.yaml
append_record "2026-01-02" "a property was added" "additive change for scenario four"
commit_all "record the additive change"
run_tool "recorded additive change passes" pass ""

start_scenario
fixture asyncapi-added.yaml asyncapi.yaml
append_record "2026-01-02" "garbage" "malformed for scenario five"
printf '%s\n' "garbage line" > "$scratch/api/breaking-changes.log"
commit_all "write a garbage record"
run_tool "malformed log line blocks" fail "not 64 hex digits"

start_scenario
fixture asyncapi-breaking.yaml asyncapi.yaml
append_record "2026-01-03" "a payload property was removed" "recorded breaking change is reported"
commit_all "record the breaking change"
run_tool "recorded breaking change passes with a report" pass "breaking protocol change"

start_scenario
append_touch
commit_all "touch the changelog comment"
run_tool "committed state passes" pass ""

if [ "$failures" -ne 0 ]; then
	say "FAIL: $failures scenario(s) blocked incorrectly"
	exit 1
fi
say "api-compat-selftest OK"
