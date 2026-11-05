#!/usr/bin/env bash
set -euo pipefail

echo "Running rustfmt check..."
(cd api && cargo fmt --all -- --check)

echo "Running clippy linter..."
(cd api && cargo clippy --workspace --all-targets -- -D warnings)

echo "Running backend test suite..."
(cd api && cargo test --workspace)

echo "Running frontend ESLint..."
(cd frontend && npm run lint)

echo "Running frontend test suite..."
(cd frontend && npm run test)

echo "All pre-commit checks passed successfully!"
