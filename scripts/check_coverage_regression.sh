#!/bin/bash
# Check coverage regression against baseline

CURRENT=$(cargo llvm-cov --quiet | grep -o "[0-9]\+%" | head -1 | sed 's/%//' || echo "0")
BASELINE_RAW=$(git show HEAD~1:coverage_report.txt 2>/dev/null || echo "0")
BASELINE=$(echo "$BASELINE_RAW" | sed 's/%//')

echo "Current coverage: $CURRENT%"
echo "Baseline coverage: $BASELINE%"

if [ -z "$BASELINE" ] || [ "$BASELINE" == "0" ]; then
    echo "WARNING: No baseline found, skipping regression check"
    echo "$CURRENT" > coverage_report.txt
    exit 0
fi

DECREASE=$(echo "$BASELINE - $CURRENT" | bc -l)

if (( $(echo "$DECREASE > 5" | bc -l) )); then
    echo "ERROR: Coverage decreased by $DECREASE% (baseline: $BASELINE%, current: $CURRENT%)"
    exit 1
fi

echo "Coverage regression check passed"
echo "$CURRENT" > coverage_report.txt
