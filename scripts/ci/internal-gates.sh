#!/usr/bin/env bash
set -euo pipefail

slice=${1:-}
base_ref=${2:-}
[[ "$slice" =~ ^slice[1-6]$ && -n "$base_ref" ]] || {
  echo 'usage: bash scripts/ci/internal-gates.sh <slice1..slice6> <base-ref>' >&2
  exit 2
}
case "$slice" in
  slice2|slice3)
    baseline_dir=$(mktemp -d "${RUNNER_TEMP:-/tmp}/hypermind-baseline.XXXXXX")
    if git show "$base_ref:eval/results/$slice.json" > "$baseline_dir/$slice.json"; then
      target/debug/hm-eval gate "$slice" --compare "$baseline_dir/$slice.json"
    else
      echo "No committed $slice baseline at $base_ref; absolute thresholds remain mandatory." >&2
      target/debug/hm-eval gate "$slice"
    fi
    ;;
  *) target/debug/hm-eval gate "$slice" --compare "$base_ref" ;;
esac
