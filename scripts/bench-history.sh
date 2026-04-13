#!/usr/bin/env bash
# bench-history.sh — Run benchmarks and append results to CSV history.
#
# Usage: ./scripts/bench-history.sh [label]
#   label  Optional tag for this run (e.g. "baseline", "post-audit")
#
# Output: bench-history.csv (appended, never truncated)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
CSV="${PROJECT_DIR}/bench-history.csv"
CC="${CC:-$HOME/.cyrius/bin/cc3}"
LABEL="${1:-$(git -C "${PROJECT_DIR}" rev-parse --short HEAD 2>/dev/null || echo 'unknown')}"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Header if file doesn't exist
if [[ ! -f "${CSV}" ]]; then
    echo "timestamp,label,benchmark,avg_us,min_us,max_us" > "${CSV}"
fi

echo "=== Argonaut Benchmark Run: ${LABEL} @ ${TIMESTAMP} ==="

# Compile bench_main
cd "${PROJECT_DIR}"
cat src/bench_main.cyr | "${CC}" 2>/dev/null > /tmp/argonaut_bench
chmod +x /tmp/argonaut_bench

# Run and capture output
BENCH_OUTPUT=$(/tmp/argonaut_bench 2>&1 || true)
echo "${BENCH_OUTPUT}"

# Parse output lines like:
#   bench_name: 4us avg (min=3us max=153us) [10000 iters]
echo "${BENCH_OUTPUT}" | grep -E 'avg.*min=.*max=' | while IFS= read -r line; do
    name=$(echo "${line}" | sed 's/:.*//' | xargs)
    avg=$(echo "${line}" | sed -n 's/.*: \([0-9]*\)us avg.*/\1/p')
    min=$(echo "${line}" | sed -n 's/.*min=\([0-9]*\)us.*/\1/p')
    max=$(echo "${line}" | sed -n 's/.*max=\([0-9]*\)us.*/\1/p')

    if [[ -n "${avg}" && -n "${name}" ]]; then
        echo "${TIMESTAMP},${LABEL},${name},${avg},${min},${max}" >> "${CSV}"
    fi
done

echo ""
echo "Results appended to ${CSV}"
