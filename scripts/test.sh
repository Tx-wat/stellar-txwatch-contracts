#!/usr/bin/env bash
# scripts/test.sh — Run all contract tests
set -euo pipefail
cargo test "$@"
