#!/usr/bin/env bash
set -uo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
here="$repo/tests/check-changed-selftest"
scratch="$repo/target/scratch/check-changed-selftest"
failures=0

say() {
	echo "check-changed-selftest: $*"
}

fail() {
	say "FAIL: $*"
	failures=$((failures + 1))
}

check_plan_case() {
	local changed="$1" name actual
	name="$(basename "$changed" .changed)"
	actual="$scratch/$name.actual"
	if ! jq -r --rawfile changed "$changed" -f "$repo/tools/check-changed-plan.jq" "$here/fixtures/metadata.json" >"$actual"; then
		fail "plan $name: jq failed"
		return 0
	fi
	if diff "$here/fixtures/cases/$name.expected" "$actual" >"$scratch/$name.diff"; then
		say "ok: plan $name"
		return 0
	fi
	fail "plan $name differs from its expected file"
	cat "$scratch/$name.diff"
}

write_fake_cargo() {
	cat >"$scratch/bin/cargo-fake" <<'EOF'
#!/usr/bin/env bash
case "$1" in
metadata) exec cargo "$@" ;;
clippy)
	if [ "$(basename "$PWD")" = "$FAKE_CLIPPY_FAILS" ]; then
		echo "fake clippy failure in $PWD" >&2
		exit 1
	fi
	exit 0
	;;
*) exit 0 ;;
esac
EOF
	printf '#!/usr/bin/env bash\nexit 0\n' >"$scratch/bin/make-fake"
	chmod +x "$scratch/bin/cargo-fake" "$scratch/bin/make-fake"
}

seed_repo() {
	local repo_dir="$scratch/repo"
	mkdir -p "$repo_dir/crates/a/src" "$repo_dir/crates/b/src" "$repo_dir/tools" "$scratch/bin"
	printf '[workspace]\nresolver = "2"\nmembers = ["crates/*"]\n' >"$repo_dir/Cargo.toml"
	printf '[package]\nname = "pkg-a"\nversion = "0.1.0"\nedition = "2024"\n' >"$repo_dir/crates/a/Cargo.toml"
	printf '[package]\nname = "pkg-b"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\npkg-a = { path = "../a" }\n' >"$repo_dir/crates/b/Cargo.toml"
	printf 'pub fn a() {}\n' >"$repo_dir/crates/a/src/lib.rs"
	printf 'pub fn b() {}\n' >"$repo_dir/crates/b/src/lib.rs"
	printf 'target/\n' >"$repo_dir/.gitignore"
	mkdir -p "$repo_dir/.config" && printf '[profile.quick]\n' >"$repo_dir/.config/nextest.toml"
	cp "$repo/tools/check-changed.sh" "$repo/tools/check-changed-plan.jq" "$repo/tools/testlock.sh" "$repo/tools/test-gate.sh" "$repo_dir/tools/"
	(cd "$repo_dir" && cargo metadata --format-version 1 --no-deps >/dev/null) || return 1
	git -C "$repo_dir" init -b main >/dev/null || return 1
	git -C "$repo_dir" config user.name selftest && git -C "$repo_dir" config user.email selftest@invalid || return 1
	git -C "$repo_dir" add -A && git -C "$repo_dir" commit -m "seed" >/dev/null
}

driver() {
	local base="$1" dry_run="$2" cargo_bin="${3:-cargo}"
	(cd "$scratch/repo" && bash tools/check-changed.sh "$cargo_bin" "$scratch/bin/make-fake" "$base" 0 "$dry_run" 2>&1)
}

expect_driver() {
	local name="$1" want_rc="$2" needle="$3" out rc
	shift 3
	out="$("$@")"
	rc=$?
	if [ "$rc" -ne "$want_rc" ]; then
		fail "$name: exit status $rc, wanted $want_rc"
		printf '%s\n' "$out"
		return 0
	fi
	if ! printf '%s' "$out" | grep -Fq -- "$needle"; then
		fail "$name: output lacks '$needle'"
		printf '%s\n' "$out"
		return 0
	fi
	say "ok: $name"
}

driver_scenarios() {
	local dir="$scratch/repo"
	expect_driver "clean tree reports nothing to check" 0 "nothing to check" driver HEAD 1
	printf 'pub fn a() { let _x = 1; }\n' >"$dir/crates/a/src/lib.rs"
	expect_driver "modified file affects the package and its dependents" 0 "affected packages (2): pkg-a pkg-b" driver HEAD 1
	git -C "$dir" checkout crates/a/src/lib.rs
	printf 'pub fn extra() {}\n' >"$dir/crates/b/src/extra.rs"
	expect_driver "untracked file affects only its own package" 0 "affected packages (1): pkg-b" driver HEAD 1
	expect_driver "unknown base is an error" 2 "unknown base" driver no-such-ref 1
	rm "$dir/crates/b/src/extra.rs"
	printf 'pub fn a() { let _y = 2; }\n' >"$dir/crates/a/src/lib.rs"
	git -C "$dir" commit -am "change a" >/dev/null
	expect_driver "clean tree against HEAD again reports nothing" 0 "nothing to check" driver HEAD 0
	expect_driver "committed change is found through the base" 0 "directly changed packages: pkg-a" driver HEAD~1 1
}

run_scenarios() {
	local out
	expect_driver "passing run prints the summary" 0 "all selected steps passed" driver HEAD~1 0 "$scratch/bin/cargo-fake"
	out="$(driver HEAD~1 0 "$scratch/bin/cargo-fake")"
	for needle in "skip" "no migrations or sqlx package source changed" "does not replace the full check"; do
		if ! printf '%s' "$out" | grep -Fq -- "$needle"; then
			fail "summary lacks '$needle'"
		fi
	done
	export FAKE_CLIPPY_FAILS=b
	expect_driver "a failing clippy package fails the run and is named" 1 "failed packages: pkg-b" driver HEAD~1 0 "$scratch/bin/cargo-fake"
	unset FAKE_CLIPPY_FAILS
	failing_test_scenario
}

failing_test_scenario() {
	local dir="$scratch/repo"
	printf 'pub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn fails() {\n        assert_eq!(1, 2);\n    }\n}\n' >"$dir/crates/a/src/lib.rs"
	expect_driver "a failing test fails the run and names its package" 1 "failed packages: pkg-a (see" driver HEAD 0
	git -C "$dir" checkout crates/a/src/lib.rs
}

mkdir -p "$repo/target/scratch"
rm -rf "$scratch"
mkdir -p "$scratch"
for changed in "$here"/fixtures/cases/*.changed; do
	check_plan_case "$changed"
done
if seed_repo; then
	write_fake_cargo
	driver_scenarios
	run_scenarios
else
	fail "could not build the scratch repository"
fi

if [ "$failures" -ne 0 ]; then
	say "FAIL: $failures check(s) failed"
	exit 1
fi
say "check-changed-selftest OK"
