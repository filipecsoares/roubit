#!/usr/bin/env bash
set -euo pipefail

echo "Running rustfmt check..."
(cd api && cargo fmt --all -- --check)

echo "Running clippy linter..."
(cd api && cargo clippy --workspace --all-targets -- -D warnings)

echo "All pre-commit checks passed successfully!"
