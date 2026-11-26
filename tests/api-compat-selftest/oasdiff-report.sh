#!/usr/bin/env bash
# oasdiff visual report for the api-compat gate: old baseline first, new revision second,
# OpenAPI only (oasdiff cannot read AsyncAPI). oasdiff exit codes:
# 0 no findings, 1 warnings only, 2 real usage error, 3 breaking changes found, 4 many breaking.
# Findings are a report (the Rust gate blocks); a usage error must fail the window.
set -uo pipefail

if [ "$#" -ne 2 ]; then
	echo "oasdiff-report: need <committed openapi> <fresh openapi>"
	exit 2
fi

out=$(oasdiff breaking "$1" "$2" --format markdown)
rc=$?
printf '%s\n' "$out"
case "$rc" in
	0 | 1 | 3 | 4) exit 0 ;;
	*)
		echo "oasdiff-report: oasdiff exited with $rc; check the command and the documents"
		exit "$rc"
		;;
esac
