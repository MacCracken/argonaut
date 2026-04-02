#!/usr/bin/env bash
# bench-history.sh — Run benchmarks and append results to CSV history.
#
# Usage: ./scripts/bench-history.sh [label]
#   label  Optional tag for this run (e.g. "baseline", "post-audit")
#
# Output: benches/history.csv (appended, never truncated)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CSV="${PROJECT_DIR}/benches/history.csv"
LABEL="${1:-$(git -C "${PROJECT_DIR}" rev-parse --short HEAD 2>/dev/null || echo 'unknown')}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Header if file doesn't exist
if [[ ! -f "${CSV}" ]]; then
    echo "timestamp,label,benchmark,ns_per_iter,low_ns,high_ns" > "${CSV}"
fi

echo "=== Argonaut Benchmark Run: ${LABEL} @ ${TIMESTAMP} ==="

# Run criterion benchmarks, capture output
BENCH_OUTPUT=$(cargo bench --bench argonaut_bench 2>&1)

echo "${BENCH_OUTPUT}"

# Parse criterion output lines like:
#   bench_name          time:   [1.2345 µs 1.3000 µs 1.3500 µs]
# or with ns units
echo "${BENCH_OUTPUT}" | grep -E 'time:.*\[' | while IFS= read -r line; do
    # Extract benchmark name (everything before "time:")
    name=$(echo "${line}" | sed 's/[[:space:]]*time:.*//' | xargs)

    # Extract the three values and unit from [low mid high]
    values=$(echo "${line}" | sed -n 's/.*\[\(.*\)\].*/\1/p')
    unit=$(echo "${values}" | grep -oE '(ns|µs|ms|s)' | head -1)

    # Extract the three numeric values
    low=$(echo "${values}" | awk '{print $1}')
    mid=$(echo "${values}" | awk '{print $3}')
    high=$(echo "${values}" | awk '{print $5}')

    # Convert to nanoseconds
    case "${unit}" in
        ns) multiplier=1 ;;
        µs) multiplier=1000 ;;
        ms) multiplier=1000000 ;;
        s)  multiplier=1000000000 ;;
        *)  multiplier=1 ;;
    esac

    # Use awk for floating-point math
    ns=$(echo "${mid} ${multiplier}" | awk '{printf "%.2f", $1 * $2}')
    low_ns=$(echo "${low} ${multiplier}" | awk '{printf "%.2f", $1 * $2}')
    high_ns=$(echo "${high} ${multiplier}" | awk '{printf "%.2f", $1 * $2}')

    if [[ -n "${ns}" && -n "${name}" ]]; then
        echo "${TIMESTAMP},${LABEL},${name},${ns},${low_ns},${high_ns}" >> "${CSV}"
    fi
done

echo ""
echo "Results appended to ${CSV}"
