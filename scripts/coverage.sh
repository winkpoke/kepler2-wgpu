#!/bin/bash
# Test coverage script for Kepler2-WGPU
# Usage: ./scripts/coverage.sh [--open] [--html-only] [--no-clean]
#   --open       Open HTML coverage report after generation
#   --html-only  Only generate HTML report (no terminal output)
#   --no-clean   Don't clean before running tests

set -e

OPEN_REPORT=false
HTML_ONLY=false
NO_CLEAN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --open)
            OPEN_REPORT=true
            shift
            ;;
        --html-only)
            HTML_ONLY=true
            shift
            ;;
        --no-clean)
            NO_CLEAN=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--open] [--html-only] [--no-clean]"
            exit 1
            ;;
    esac
done

if [ "$NO_CLEAN" = false ]; then
    echo "Cleaning previous build artifacts..."
    cargo clean
fi

echo "Running test coverage..."
cargo llvm-cov --html --output-dir coverage

echo ""
echo "Coverage report generated: coverage/index.html"

if [ "$OPEN_REPORT" = true ]; then
    echo "Opening coverage report..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        open coverage/index.html
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        xdg-open coverage/index.html
    else
        echo "Please open coverage/index.html in your browser"
    fi
fi
