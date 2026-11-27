#!/usr/bin/env bash
set -uo pipefail

cargo="${1:-cargo}"
make_cmd="${2:-make}"
base="${3:-HEAD}"
with_pg="${4:-0}"
dry_run="${5:-0}"
root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/target/check-changed"
tab=$'\t'
total_start=$(date +%s)

say() {
	echo "check-changed: $*"
}

cloud_note() {
	say "this fast loop is derived from changed files and does not replace the full check; the full make check is still done by the cloud check"
}

collect_changed() {
	if ! git rev-parse --verify --quiet "$base^{commit}" >/dev/null; then
		say "unknown base '$base' (set CHANGED_BASE to a commit, branch, or tag)" >&2
		return 2
	fi
	{
		git diff --name-only --no-renames "$base" --
		git ls-files --others --exclude-standard
	} | sort -u >"$out/files.txt"
}

make_plan() {
	"$cargo" metadata --format-version 1 --no-deps >"$out/metadata.json" || return 1
	jq -r --rawfile changed "$out/files.txt" -f "$root/tools/check-changed-plan.jq" "$out/metadata.json" >"$out/plan.txt"
}

plan_column() {
	awk -F'\t' -v kind="$1" -v col="$2" '$1 == kind { print $col }' "$out/plan.txt"
}

print_plan() {
	say "base $base, $(plan_column changed 2) changed files"
	say "directly changed packages: $(plan_column direct 2 | paste -sd' ' -)"
	say "affected packages ($(plan_column affected 2 | wc -l | tr -d ' ')): $(plan_column affected 2 | paste -sd' ' -)"
	say "plan:"
	awk -F'\t' '$1 == "step" { printf "  %-24s %-5s %s\n", $2, $3, $4 }' "$out/plan.txt"
}

record() {
	printf '%s\t%s\t%s\t%s\n' "$1" "$2" "$3" "$4" >>"$out/results.tsv"
}

finish_step() {
	local name="$1" rc="$2" start="$3" note="$4" status="ok"
	if [ "$rc" -ne 0 ]; then
		status="FAILED"
	fi
	record "$name" "$status" "$(($(date +%s) - start))" "$note"
}

run_step() {
	local name="$1" note="$2" start rc
	shift 2
	echo ">>> $name: $*"
	start=$(date +%s)
	"$@"
	rc=$?
	finish_step "$name" "$rc" "$start" "$note"
}

step_clippy() {
	local start failed="" kind name dir count=0
	start=$(date +%s)
	while IFS=$'\t' read -r -u 4 kind name dir; do
		[ "$kind" = affected ] || continue
		count=$((count + 1))
		echo ">>> clippy: $name ($dir)"
		(cd "$root/$dir" && "$cargo" clippy --all-targets -- -D warnings) || failed="$failed $name"
	done 4<"$out/plan.txt"
	if [ -n "$failed" ]; then
		finish_step clippy 1 "$start" "failed packages:$failed"
		return 0
	fi
	finish_step clippy 0 "$start" "$count packages"
}

failed_test_packages() {
	sed -nE 's/^ *(FAIL|SIGABRT|SIGSEGV|TIMEOUT|LEAK-FAIL|EXECFAIL) *\[[^]]*\] +(\([0-9]+\/[0-9]+\) +)?([^ ]+).*/\3/p' "$out/nextest.log" |
		sed 's/::.*//' | sort -u | paste -sd' ' -
}

step_test() {
	local start rc name packages=() selection
	start=$(date +%s)
	while IFS= read -r name; do
		packages+=(-p "$name")
	done < <(plan_column affected 2)
	if [ "$with_pg" = 1 ]; then
		selection=(-E 'not test(/_compile_fail$/)')
	else
		selection=(--profile quick)
	fi
	echo ">>> test: cargo nextest run ${selection[*]} ${packages[*]}"
	bash "$root/tools/testlock.sh" bash "$root/tools/test-gate.sh" run "$out/nextest.log" \
		"$cargo" nextest run --no-tests=pass --no-fail-fast "${selection[@]}" "${packages[@]}"
	rc=$?
	if [ "$rc" -ne 0 ]; then
		finish_step test 1 "$start" "failed packages: $(failed_test_packages) (see $out/nextest.log)"
		return 0
	fi
	finish_step test 0 "$start" "$([ "$with_pg" = 1 ] && echo "with PostgreSQL tests" || echo "quick profile, PostgreSQL tests excluded")"
}

execute_plan() {
	local kind step state reason
	while IFS=$'\t' read -r -u 3 kind step state reason; do
		[ "$kind" = step ] || continue
		if [ "$state" = skip ]; then
			record "$step" skip - "$reason"
			continue
		fi
		case "$step" in
		fmt) run_step fmt "cargo fmt --all --check" "$cargo" fmt --all --check ;;
		clippy) step_clippy ;;
		test) step_test ;;
		*) run_step "$step" "make $step ($reason)" "$make_cmd" --no-print-directory "$step" ;;
		esac
	done 3<"$out/plan.txt"
}

print_summary() {
	echo
	say "summary (base $base, $(plan_column changed 2) changed files, $(plan_column affected 2 | wc -l | tr -d ' ') affected packages)"
	printf '%-24s %-7s %6s  %s\n' step result secs note
	while IFS=$'\t' read -r name status secs note; do
		printf '%-24s %-7s %6s  %s\n' "$name" "$status" "$secs" "$note"
	done <"$out/results.tsv"
	say "total $(($(date +%s) - total_start))s"
}

main() {
	mkdir -p "$out" && cd "$root" || return 2
	collect_changed || return $?
	if [ ! -s "$out/files.txt" ]; then
		say "no changes against $base (no modified, staged, or untracked files); nothing to check"
		cloud_note
		return 0
	fi
	make_plan || return 1
	print_plan
	if [ "$dry_run" = 1 ]; then
		say "dry run: nothing executed"
		return 0
	fi
	: >"$out/results.tsv"
	execute_plan
	print_summary
	cloud_note
	if awk -F'\t' '$2 == "FAILED" { found = 1 } END { exit !found }' "$out/results.tsv"; then
		say "FAILED: $(awk -F'\t' '$2 == "FAILED" { printf "%s ", $1 }' "$out/results.tsv")" >&2
		return 1
	fi
	say "all selected steps passed"
}

main
