#!/bin/bash
set -uo pipefail

forbidden_phrase="ignoring unknown configuration keys"

run_gate() {
	log_file="$1"
	shift
	if ! "$@" 2>&1 | tee "$log_file"; then
		echo "test-gate: command exit status is not zero" >&2
		return 1
	fi
	if grep -q "$forbidden_phrase" "$log_file"; then
		echo "test-gate: log contains the forbidden phrase" >&2
		return 1
	fi
	return 0
}

usage_gate() {
	echo "usage: test-gate.sh run|expect-fail|expect-pass <log-file> command..." >&2
	exit 2
}

main_gate() {
	if [ "$#" -lt 3 ]; then
		usage_gate
	fi
	mode="$1"
	log_file="$2"
	shift 2
	case "$mode" in
		run)
			run_gate "$log_file" "$@"
			;;
		expect-fail)
			if run_gate "$log_file" "$@"; then
				echo "gate-selftest: gate accepted a command that must be blocked" >&2
				exit 1
			fi
			;;
		expect-pass)
			if ! run_gate "$log_file" "$@"; then
				echo "gate-selftest: gate blocked a clean command" >&2
				exit 1
			fi
			;;
		*)
			usage_gate
			;;
	esac
}

main_gate "$@"
